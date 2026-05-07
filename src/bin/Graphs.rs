use plotters::prelude::*;
use std::error::Error;
use std::fs;

#[derive(Debug)]
struct MonitorRow {
    time_ms: u32,
    cpu_used_percent: u32,
    active_workers: u32,
    queue_len: u32,
    completed: u32,
}

struct Metric {
    title: &'static str,
    y_label: &'static str,
    output_file: &'static str,
    value: fn(&MonitorRow) -> u32,
}

fn main() -> Result<(), Box<dyn Error>> {
    let fifo_rows = load_monitor_data("monitor_fifo.csv")?;
    let optimized_rows = load_monitor_data("monitor_optimized.csv")?;

    fs::create_dir_all("graphs")?;

    let metrics = [
        Metric {
            title: "CPU Usage Over Time",
            y_label: "CPU Used (%)",
            output_file: "graphs/cpu_usage_comparison.png",
            value: |row| row.cpu_used_percent,
        },
        Metric {
            title: "Active Workers Over Time",
            y_label: "Active Workers",
            output_file: "graphs/active_workers_comparison.png",
            value: |row| row.active_workers,
        },
        Metric {
            title: "Queue Length Over Time",
            y_label: "Queue Length",
            output_file: "graphs/queue_length_comparison.png",
            value: |row| row.queue_len,
        },
        Metric {
            title: "Completed Tasks Over Time",
            y_label: "Completed Tasks",
            output_file: "graphs/completed_tasks_comparison.png",
            value: |row| row.completed,
        },
    ];

    for metric in metrics {
        draw_comparison_graph(&fifo_rows, &optimized_rows, metric)?;
    }

    println!("Graphs created in the graphs folder:");
    println!("  graphs/cpu_usage_comparison.png");
    println!("  graphs/active_workers_comparison.png");
    println!("  graphs/queue_length_comparison.png");
    println!("  graphs/completed_tasks_comparison.png");

    Ok(())
}

fn load_monitor_data(path: &str) -> Result<Vec<MonitorRow>, Box<dyn Error>> {
    let file_text = fs::read_to_string(path)?;
    let mut rows = Vec::new();

    for line in file_text.lines().skip(1) {
        let values: Vec<&str> = line.split(',').map(|value| value.trim()).collect();

        if values.len() != 5 {
            continue;
        }

        let row = MonitorRow {
            time_ms: values[0].parse()?,
            cpu_used_percent: values[1].parse()?,
            active_workers: values[2].parse()?,
            queue_len: values[3].parse()?,
            completed: values[4].parse()?,
        };

        rows.push(row);
    }

    Ok(rows)
}

fn draw_comparison_graph(
    fifo_rows: &[MonitorRow],
    optimized_rows: &[MonitorRow],
    metric: Metric,
) -> Result<(), Box<dyn Error>> {
    let fifo_points = make_points(fifo_rows, metric.value);
    let optimized_points = make_points(optimized_rows, metric.value);

    let max_time = fifo_rows
        .iter()
        .chain(optimized_rows.iter())
        .map(|row| row.time_ms)
        .max()
        .unwrap_or(1);

    let highest_value = fifo_rows
        .iter()
        .chain(optimized_rows.iter())
        .map(metric.value)
        .max()
        .unwrap_or(1);

    let y_padding = (highest_value / 10).max(1);
    let max_y = highest_value + y_padding;

    let root = BitMapBackend::new(metric.output_file, (1000, 600)).into_drawing_area();
    root.fill(&WHITE)?;

    let mut chart = ChartBuilder::on(&root)
        .caption(metric.title, ("sans-serif", 30))
        .margin(20)
        .x_label_area_size(45)
        .y_label_area_size(55)
        .build_cartesian_2d(0u32..max_time, 0u32..max_y)?;

    chart
        .configure_mesh()
        .x_desc("Time (ms)")
        .y_desc(metric.y_label)
        .draw()?;

    chart
        .draw_series(LineSeries::new(fifo_points, BLUE))?
        .label("FIFO")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 25, y)], BLUE));

    chart
        .draw_series(LineSeries::new(optimized_points, RED))?
        .label("Optimized")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 25, y)], RED));

    chart
        .configure_series_labels()
        .background_style(WHITE)
        .border_style(BLACK)
        .draw()?;

    root.present()?;

    Ok(())
}

fn make_points(rows: &[MonitorRow], value_function: fn(&MonitorRow) -> u32) -> Vec<(u32, u32)> {
    rows.iter()
        .map(|row| (row.time_ms, value_function(row)))
        .collect()
}