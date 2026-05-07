use rand::{rngs::StdRng, RngExt, SeedableRng};
use std::collections::VecDeque;
use std::fs::File;
use std::io::{self, Write};

const NUM_TASKS: usize = 1000;
const IO_PROBABILITY: f64 = 0.70;

const ARRIVAL_INTERVAL_MS: u32 = 20;
const TASK_DURATION_MS: u32 = 200;
const MONITOR_INTERVAL_MS: u32 = 10;

const MAX_WORKERS: usize = 8;
const CPU_LIMIT_PERCENT: u32 = 100;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TaskKind {
    Io,
    Cpu,
}

impl TaskKind {
    fn cpu_cost(self) -> u32 {
        match self {
            TaskKind::Io => 10,
            TaskKind::Cpu => 35,
        }
    }
}

#[derive(Clone, Debug)]
struct Task {
    id: usize,
    kind: TaskKind,
    arrival_ms: u32,
}

#[derive(Clone, Debug)]
struct RunningTask {
    task: Task,
    start_ms: u32,
    end_ms: u32,
}

#[derive(Clone, Debug)]
struct Snapshot {
    time_ms: u32,
    cpu_used_percent: u32,
    active_workers: usize,
    queue_len: usize,
    completed: usize,
}

#[derive(Clone, Copy, Debug)]
enum Strategy {
    Fifo,
    Optimized,
}

impl Strategy {
    fn name(self) -> &'static str {
        match self {
            Strategy::Fifo => "FIFO",
            Strategy::Optimized => "Optimized",
        }
    }
}

struct SimulationResult {
    strategy: Strategy,
    total_runtime_ms: u32,
    makespan_ms: u32,

    completed: usize,
    io_completed: usize,
    cpu_completed: usize,

    avg_wait_ms: f64,
    avg_wait_io_ms: f64,
    avg_wait_cpu_ms: f64,
    avg_turnaround_ms: f64,

    max_wait_ms: u32,
    max_wait_task_id: usize,

    avg_cpu_percent: f64,
    avg_workers: f64,

    monitor_csv: String,
    snapshots: Vec<Snapshot>,
}

fn main() -> io::Result<()> {
    let seed = 42;
    let tasks = generate_tasks(seed);

    let generated_io = tasks.iter().filter(|task| task.kind == TaskKind::Io).count();
    let generated_cpu = tasks.iter().filter(|task| task.kind == TaskKind::Cpu).count();

    println!("Generated task mix using seed {}:", seed);
    println!("  IO tasks: {}", generated_io);
    println!("  CPU tasks: {}", generated_cpu);
    println!();

    let fifo = run_simulation(&tasks, Strategy::Fifo);
    write_csv(&fifo.monitor_csv, &fifo)?;
    print_summary(&fifo);

    println!();

    let optimized = run_simulation(&tasks, Strategy::Optimized);
    write_csv(&optimized.monitor_csv, &optimized)?;
    print_summary(&optimized);

    println!();
    print_comparison(&fifo, &optimized);

    Ok(())
}

fn generate_tasks(seed: u64) -> Vec<Task> {
    let mut rng = StdRng::seed_from_u64(seed);
    let mut tasks = Vec::with_capacity(NUM_TASKS);

    for i in 0..NUM_TASKS {
        let kind = if rng.random_bool(IO_PROBABILITY) {
            TaskKind::Io
        } else {
            TaskKind::Cpu
        };

        tasks.push(Task {
            id: i,
            kind,
            arrival_ms: i as u32 * ARRIVAL_INTERVAL_MS,
        });
    }

    tasks
}

fn run_simulation(tasks: &[Task], strategy: Strategy) -> SimulationResult {
    let mut time_ms = 0;
    let mut next_arrival = 0;

    let mut queue: VecDeque<Task> = VecDeque::new();
    let mut workers: Vec<Option<RunningTask>> = vec![None; MAX_WORKERS];

    let mut snapshots = Vec::new();

    let mut completed = 0;
    let mut io_completed = 0;
    let mut cpu_completed = 0;

    let mut wait_sum: u64 = 0;
    let mut wait_io_sum: u64 = 0;
    let mut wait_cpu_sum: u64 = 0;
    let mut turnaround_sum: u64 = 0;

    let mut max_wait_ms = 0;
    let mut max_wait_task_id = 0;

    let first_arrival_ms = tasks.first().map_or(0, |task| task.arrival_ms);
    let mut last_finish_ms = 0;

    loop {
        while next_arrival < tasks.len() && tasks[next_arrival].arrival_ms <= time_ms {
            queue.push_back(tasks[next_arrival].clone());
            next_arrival += 1;
        }

        for slot in workers.iter_mut() {
            let is_done = slot
                .as_ref()
                .map_or(false, |running| running.end_ms <= time_ms);

            if is_done {
                let finished = slot.take().unwrap();

                let wait_ms = finished.start_ms - finished.task.arrival_ms;
                let turnaround_ms = finished.end_ms - finished.task.arrival_ms;

                wait_sum += wait_ms as u64;
                turnaround_sum += turnaround_ms as u64;

                if wait_ms > max_wait_ms {
                    max_wait_ms = wait_ms;
                    max_wait_task_id = finished.task.id;
                }

                last_finish_ms = last_finish_ms.max(finished.end_ms);

                completed += 1;

                match finished.task.kind {
                    TaskKind::Io => {
                        io_completed += 1;
                        wait_io_sum += wait_ms as u64;
                    }
                    TaskKind::Cpu => {
                        cpu_completed += 1;
                        wait_cpu_sum += wait_ms as u64;
                    }
                }
            }
        }

        loop {
            let Some(worker_id) = workers.iter().position(|slot| slot.is_none()) else {
                break;
            };

            let cpu_used = current_cpu_used(&workers);

            if cpu_used >= CPU_LIMIT_PERCENT {
                break;
            }

            let cpu_left = CPU_LIMIT_PERCENT - cpu_used;

            let Some(task_index) = choose_task_index(&queue, &workers, strategy, cpu_left) else {
                break;
            };

            let task = queue.remove(task_index).unwrap();

            workers[worker_id] = Some(RunningTask {
                task,
                start_ms: time_ms,
                end_ms: time_ms + TASK_DURATION_MS,
            });
        }

        snapshots.push(Snapshot {
            time_ms,
            cpu_used_percent: current_cpu_used(&workers),
            active_workers: active_worker_count(&workers),
            queue_len: queue.len(),
            completed,
        });

        if completed == tasks.len() {
            break;
        }

        time_ms += MONITOR_INTERVAL_MS;
    }

    let sample_count = snapshots.len().max(1) as f64;

    let avg_cpu_percent = snapshots
        .iter()
        .map(|s| s.cpu_used_percent as f64)
        .sum::<f64>()
        / sample_count;

    let avg_workers = snapshots
        .iter()
        .map(|s| s.active_workers as f64)
        .sum::<f64>()
        / sample_count;

    SimulationResult {
        strategy,
        total_runtime_ms: time_ms,
        makespan_ms: last_finish_ms - first_arrival_ms,

        completed,
        io_completed,
        cpu_completed,

        avg_wait_ms: average(wait_sum, completed),
        avg_wait_io_ms: average(wait_io_sum, io_completed),
        avg_wait_cpu_ms: average(wait_cpu_sum, cpu_completed),
        avg_turnaround_ms: average(turnaround_sum, completed),

        max_wait_ms,
        max_wait_task_id,

        avg_cpu_percent,
        avg_workers,

        monitor_csv: csv_filename(strategy).to_string(),
        snapshots,
    }
}

fn choose_task_index(
    queue: &VecDeque<Task>,
    workers: &[Option<RunningTask>],
    strategy: Strategy,
    cpu_left: u32,
) -> Option<usize> {
    match strategy {
        Strategy::Fifo => {
            let front = queue.front()?;

            if front.kind.cpu_cost() <= cpu_left {
                Some(0)
            } else {
                None
            }
        }

        Strategy::Optimized => {
            let running_cpu = running_kind_count(workers, TaskKind::Cpu);
            let running_io = running_kind_count(workers, TaskKind::Io);

            // Target pattern:
            // 2 CPU tasks = 70% CPU
            // 3 IO tasks  = 30% CPU
            // Total        = 100% CPU
            if running_cpu < 2 {
                if let Some(index) = find_task_index_by_kind(queue, TaskKind::Cpu, cpu_left) {
                    return Some(index);
                }
            }

            if running_io < 3 {
                if let Some(index) = find_task_index_by_kind(queue, TaskKind::Io, cpu_left) {
                    return Some(index);
                }
            }

            // If CPU tasks are waiting but one cannot fit right now,
            // stop adding extra IO tasks so CPU tasks do not starve.
            if queue_has_kind(queue, TaskKind::Cpu) && cpu_left < TaskKind::Cpu.cpu_cost() {
                return None;
            }

            // Fallback: use IO tasks if possible.
            if let Some(index) = find_task_index_by_kind(queue, TaskKind::Io, cpu_left) {
                return Some(index);
            }

            // Last fallback: run a CPU task if it fits.
            find_task_index_by_kind(queue, TaskKind::Cpu, cpu_left)
        }
    }
}

fn running_kind_count(workers: &[Option<RunningTask>], kind: TaskKind) -> usize {
    workers
        .iter()
        .filter_map(|slot| slot.as_ref())
        .filter(|running| running.task.kind == kind)
        .count()
}

fn queue_has_kind(queue: &VecDeque<Task>, kind: TaskKind) -> bool {
    queue.iter().any(|task| task.kind == kind)
}

fn find_task_index_by_kind(
    queue: &VecDeque<Task>,
    kind: TaskKind,
    cpu_left: u32,
) -> Option<usize> {
    queue
        .iter()
        .position(|task| task.kind == kind && task.kind.cpu_cost() <= cpu_left)
}

fn current_cpu_used(workers: &[Option<RunningTask>]) -> u32 {
    workers
        .iter()
        .filter_map(|slot| slot.as_ref())
        .map(|running| running.task.kind.cpu_cost())
        .sum()
}

fn active_worker_count(workers: &[Option<RunningTask>]) -> usize {
    workers.iter().filter(|slot| slot.is_some()).count()
}

fn average(sum: u64, count: usize) -> f64 {
    if count == 0 {
        0.0
    } else {
        sum as f64 / count as f64
    }
}

fn csv_filename(strategy: Strategy) -> &'static str {
    match strategy {
        Strategy::Fifo => "monitor_fifo.csv",
        Strategy::Optimized => "monitor_optimized.csv",
    }
}

fn print_summary(result: &SimulationResult) {
    println!("== {} simulation ==", result.strategy.name());
    println!(
        "{} tasks, target {:.0}% IO / {:.0}% CPU, {} workers, cap {}%",
        NUM_TASKS,
        IO_PROBABILITY * 100.0,
        (1.0 - IO_PROBABILITY) * 100.0,
        MAX_WORKERS,
        CPU_LIMIT_PERCENT
    );

    println!();
    println!("-- results --");

    println!("{:<25}: {} ms", "total runtime", result.total_runtime_ms);
    println!("{:<25}: {} ms", "makespan", result.makespan_ms);

    println!(
        "{:<25}: {} (IO={}, CPU={})",
        "tasks completed", result.completed, result.io_completed, result.cpu_completed
    );

    println!("{:<25}: {:.2} ms", "avg wait time", result.avg_wait_ms);

    if matches!(result.strategy, Strategy::Optimized) {
        println!(
            "{:<25}: {:.2} ms",
            "avg wait (IO only)", result.avg_wait_io_ms
        );
        println!(
            "{:<25}: {:.2} ms",
            "avg wait (CPU only)", result.avg_wait_cpu_ms
        );
    }

    println!(
        "{:<25}: {:.2} ms",
        "avg turnaround time", result.avg_turnaround_ms
    );

    println!(
        "{:<25}: {} ms (task #{})",
        "max wait time", result.max_wait_ms, result.max_wait_task_id
    );

    println!("{:<25}: {:.2} %", "avg CPU usage", result.avg_cpu_percent);

    println!(
        "{:<25}: {:.2} / {}",
        "avg workers active", result.avg_workers, MAX_WORKERS
    );

    println!("{:<25}: {}", "monitor samples", result.snapshots.len());
    println!("{:<25}: {}", "monitor csv", result.monitor_csv);
}

fn print_comparison(fifo: &SimulationResult, optimized: &SimulationResult) {
    let runtime_saved = fifo.total_runtime_ms as i32 - optimized.total_runtime_ms as i32;
    let runtime_improvement_percent =
        runtime_saved as f64 / fifo.total_runtime_ms as f64 * 100.0;

    let cpu_usage_gain = optimized.avg_cpu_percent - fifo.avg_cpu_percent;

    println!("== Comparison Summary ==");
    println!(
        "Runtime improvement     : {} ms faster ({:.2}%)",
        runtime_saved, runtime_improvement_percent
    );
    println!("Average CPU usage gain  : {:.2}%", cpu_usage_gain);
    println!("FIFO total runtime      : {} ms", fifo.total_runtime_ms);
    println!(
        "Optimized total runtime : {} ms",
        optimized.total_runtime_ms
    );
    println!("FIFO avg CPU usage      : {:.2}%", fifo.avg_cpu_percent);
    println!(
        "Optimized avg CPU usage : {:.2}%",
        optimized.avg_cpu_percent
    );
}

fn write_csv(path: &str, result: &SimulationResult) -> io::Result<()> {
    let mut file = File::create(path)?;

    writeln!(
        file,
        "time_ms,cpu_used_percent,active_workers,queue_len,completed"
    )?;

    for s in &result.snapshots {
        writeln!(
            file,
            "{},{},{},{},{}",
            s.time_ms, s.cpu_used_percent, s.active_workers, s.queue_len, s.completed
        )?;
    }

    Ok(())
}