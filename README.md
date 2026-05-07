===============================================================
                    CPU Dispatcher Simulation
===============================================================

This project is a simulation of a task dispatcher using a CPU. It compares two scheduling strategies:

FIFO Scheduler
    - Processes tasks in the order in which they arrive.
    - If the first task in the queue cannot fit under the CPU limit, the dispatcher waits.

Optimized Scheduler
    - Looks through the queue for a task that better fits the remaining CPU capacity.
    - Tries to improve total runtime and average CPU usage by combining CPU-heavy and IO-heavy tasks more efficiently.

In this project, each task has:

a task ID,
a task type, either IO or CPU,
an arrival time,
and a fixed duration of 200 ms.

These are the CPU costs in the dispatcher:

| Task Type   ||  CPU Cost |
============================
| IO Task     ||    10%    |
| CPU Task    ||    35%    |

Simulation settings:

| Setting                 || Value  |
====================================
| # of tasks              || 1000   |
| Task arrival interval   || 20 ms  |
| Task duration           || 200 ms |
| Monitor interval        || 10 ms  |
| Max worker limit        || 8      |
| CPU limit               || 100%   |

===============================================================
                         Run/Build Instructions
===============================================================

Ensure that Rust and Cargo are both installed.

To build the project:

            cargo build

To run the project:

            cargo run

To run the project without extra Cargo output:

            cargo run --quiet

To save the printed experiment output into a text file:

            cargo run --quiet > experiment_output.txt

===============================================================
                            Output Files
===============================================================

The system creates the following output files:

experiment_output.txt
monitor_fifo.csv
monitor_optimized.csv

The experiment_output.txt file contains printed metrics from both simulations.

The CSV files contain monitor samples from each simulation, including:

- Time in milliseconds
- CPU usage percentage
- Active workers
- Queue length
- Completed task count

===============================================================
                        Experiment Results
===============================================================

The main comparison is between total runtime and average CPU usage.

The following are the final results from one run:

| Metric              || FIFO      || Optimized        |
=====================================================
| Total runtime       || 39040 ms  || 36320 ms         |
| Avg CPU usage       || 89.50%    || 96.20%           |
| Runtime improvement || N/A       || 2720 ms faster   |
| CPU usage gain      || N/A       || 6.70%            |

The optimized scheduler completed the same workload faster and used the CPU more efficiently than the FIFO scheduling strategy.

===============================================================
                         Design Strategy
===============================================================

This simulation runs in 10 ms time steps.

At each step:

A new task arrives and enters the manager queue.
Finished worker tasks are marked as complete.
The dispatcher assigns waiting tasks to available workers.
The monitor records CPU usage, active workers, queue length, and completed task count.

The manager queue is stored using a VecDeque.

The workers are stored as a vector of optional running tasks.
If a worker slot is empty, then the dispatcher can assign a task to that worker.

===============================================================
                         Scheduling Policy
===============================================================
The FIFO scheduler only checks the first task in the queue. If that task fits within the remaining CPU capacity, it runs. However, if it does not fit, the First In First Out scheduling system waits.

The optimized scheduler checks the queue for a task that better fits the remaining CPU budget and then tries to use efficient combinations such as:

2 CPU tasks + 3 IO tasks = 100% CPU usage

This is because:

2 CPU tasks = 70%
3 IO tasks = 30% +
__________________
100% CPU usage

This strategy helps keep the CPU usage closer to the 100% CPU cap.

===============================================================
                        Synchronization Strategy
===============================================================

The manager queue, workers, and monitor are modeled inside one controlled simulation loop. This means the update order is fixed:

Add arrived tasks to the queue.
Complete finished tasks.
Dispatch waiting tasks.
Record monitor data.

Race conditions are prevented by this fixed order in the simulation.

In a real threaded version, the manager queue would need synchronization tools such as a mutex and/or a condition variable.

===============================================================
                         Trade-Offs
===============================================================

A FIFO scheduling strategy is simple and fair by arrival order. However, it can leave CPU capacity unused if the front task does not fit.

The optimized scheduler improves CPU usage and total runtime, but it is less strict about arrival order. This means that some tasks may wait longer because the scheduler chooses tasks that better fit the CPU budget.

This project shows the trade-off between fairness and resource utilization between both scheduling strategies.

===============================================================
                         Rand and Its Use
===============================================================

This project uses the rand crate to generate random seeded tasks.

Each task is generated with:

70% chance = IO task
30% chance = CPU task

A fixed random seed is used so that the results are repeatable. This also makes the FIFO and Optimized comparison fair because both strategies run on the same task list.

===============================================================
                          Expected Files
===============================================================
Expect these files:

src/main.rs
Cargo.toml
Cargo.lock
README.md
experiment_output.txt
monitor_fifo.csv
monitor_optimized.csv
CPU_Dispatcher_Design_Report.pdf
Graph files aswell

===============================================================
                          Tool Usage
===============================================================

I used the following tools to make the project as accurate as I could:

GitHub and GitHub Codespaces for editing, running, and managing the project.

The rand crate for random task generation.

AI tools like Claude and ChatGPT for debugging help and README/report preparation.

===============================================================
                          Tool Use Disclosure
===============================================================

One piece of advice that I accepted was to add a final comparison summary to the program output. This made it easier to compare results because the runtime improvement, CPU usage gain, total runtime, and average CPU usage were clearly printed.

One piece of advice that I rejected was adding a more complicated fairness and/or aging system to the optimized scheduler. While that could reduce wait time for some tasks, it could also make the scheduler harder to explain and might reduce the CPU usage improvement. I kept the optimization policy focused on improving total runtime and average CPU usage without overcomplicating the design.
