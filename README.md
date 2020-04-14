# Rust async-await

A Rust demo program showing the basic components of asynchrony in the language i.e

1. **Future**: *A Unit of computation*

```rust
struct TimerFuture {
shared_state: Arc<Mutex<SharedState>>,
}

struct SharedState {
completed: bool,
			   waker: Option<Waker>,
}

impl Future for TimerFuture {
	type Output = ();
	fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		let mut shared_state = self.shared_state.lock().unwrap();
		if shared_state.completed {
			Poll::Ready(())
		} else {
			// Create the waker
			shared_state.waker = Some(cx.waker().clone());
			Poll::Pending
		}
	}
}

impl TimerFuture {
	pub fn new(duration: Duration) -> Self {
		let shared_state = Arc::new(Mutex::new(SharedState {
completed: false,
waker: None,
}));
...
TimerFuture { shared_state }
}
```
2. **Executor**: *Offloads work off a queue and runs them*

```rust
struct Executor {
ready_queue: Receiver<Arc<Task>>,
}

impl Executor {
	fn run(&self) {
		while let Ok(task) = self.ready_queue.recv() {
			.....
		}
	}
}

```
3. **Reactor**: *Executes tasks are ready to execute*

```rust
impl ArcWake for Task {
	fn wake_by_ref(arc_self: &Arc<Self>) {
		let cloned = arc_self.clone();
		...
			arc_self.task_sender.send(cloned).expect("too many tasks queued")
	}
}
```

4. **Generator**(stackless routines) : *Currently unstable*

```rust
// Just to understand generators!
fn genums() -> impl std::ops::Generator<Yield = u32, Return = ()> {
	|| {
		let xs: Vec<u32> = (1..10).collect();
		...
		yield sum;
		}
}
```
5. **Spawner** : *Spawns new stuff onto the executor!*

```rust
#[derive(Clone)]
struct Spawner {
task_sender: SyncSender<Arc<Task>>,
}

fn new_executor_and_spawner() -> (Executor, Spawner) {
	// Max allowable tasks on the Queue
	const MAX_Q_TASKS: usize = 10_000;
	let (task_sender, ready_queue) = sync_channel(MAX_Q_TASKS);
	(Executor { ready_queue }, Spawner { task_sender })
}

```
6. **Task** : *A chain of futures or Unit of work*

```rust
// A future can reschedule itself to be polled by an executor
struct Task {
	// In progress future to be pushed to completion
future: Mutex<Option<BoxFuture<'static, ()>>>,
			// Handle to place task itself back onto the task queue
			task_sender: SyncSender<Arc<Task>>,
}
```

Adapted from : [This talk](https://www.youtube.com/watch?v=NNwK5ZPAJCk) and [This sources](https://github.com/rust-lang/async-book/blob/master/examples/02_04_executor/src/lib.rs)

## Usage
Use the following commands

```bash
$ cargo build
$ cargo run
```
