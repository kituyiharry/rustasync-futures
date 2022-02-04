//#![feature(generator_trait)]
//#![feature(generators)]
extern crate futures;
type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + 'a + Send>>;
//type BoxFuture<'a, T> = Box<dyn Future<Output = T> + 'a + Send>; // https://docs.rs/futures/0.1.5/futures/future/type.BoxFuture.html
//
//https://github.com/rust-lang/async-book/blob/master/examples/02_04_executor/src/lib.rs

use futures::task::{waker_ref, ArcWake};
use std::future::Future;

// Before a future starts executing we need to move it around in memory
// i.e stack to heap
// But rust has no GC, Thus moving it with pointers will invalidate any pointers and BOOM!
//
// Pin on pointer, will no longer move....forever!!
// Pin is the language for borrowing in futures i think
// UnPin is opposite!!
use std::pin::Pin;
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::{Arc, Mutex};
use std::task::Context;
use std::task::Poll;
use std::task::Waker;
use std::time::Duration;

/*
 *  Task: A unit of work to exec, A chain of futures
 *  Executor: Schedules tasks
 *  Reactor: Notifies executor tasks are ready to execute
 * */

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

        let thread_shared_state = shared_state.clone();
        std::thread::spawn(move || {
            std::thread::sleep(duration);
            let mut shared_state = thread_shared_state.lock().unwrap();
            shared_state.completed = true;
            if let Some(waker) = shared_state.waker.take() {
                waker.wake();
            }
        });

        TimerFuture { shared_state }
    }
}

// Our Executor receives tasks off a channel and runs them
struct Executor {
    ready_queue: Receiver<Arc<Task>>,
}

// A future can reschedule itself to be polled by an executor
// Multi-threaded! -> This is a simplified example so dont worry
struct Task {
    // In progress future to be pushed to completion
    future: Mutex<Option<BoxFuture<'static, ()>>>,
    // Handle to place task itself back onto the task queue
    task_sender: SyncSender<Arc<Task>>,
}

// A spawner spawns new futures onto the task channel
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

// Sends a task to the executor
impl Spawner {
    fn spawn(&self, future: impl Future<Output = ()> + 'static + Send) {
        let future = Pin::from(Box::new(future)); // https://github.com/rust-lang/futures-rs/issues/228
        let task = Arc::new(Task {
            future: Mutex::new(Some(future)),
            task_sender: self.task_sender.clone(),
        });
        self.task_sender.send(task).expect("Too many tasks queued");
    }
}

// Whenever a future calls wake, aend it back to that channel
// This is the part where the reactor comes in
// Think of the MIO stuff!!
impl ArcWake for Task {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        let cloned = arc_self.clone();
        arc_self
            .task_sender
            .send(cloned)
            .expect("too many tasks queued")
    }
}

impl Executor {
    fn run(&self) {
        while let Ok(task) = self.ready_queue.recv() {
            let mut future_slot = task.future.lock().unwrap();

            if let Some(mut future) = future_slot.take() {
                let waker = waker_ref(&task);
                let ctx = &mut Context::from_waker(&*waker);
                if let Poll::Pending = future.as_mut().poll(ctx) {
                    *future_slot = Some(future)
                }
            }
        }
    }
}

// Just to understand generators!
// fn genums() -> impl std::ops::Generator<Yield = u32, Return = ()> {
//     || {
//         let xs: Vec<u32> = (1..10).collect();
//         let mut sum = 0;
//         for i in xs {
//             sum += i;
//             yield sum;
//         }
//     }
// }

// Rules of async/await
// Async fn return some sort of anon future struct
// Call .await on a future to get the value
// Call .await only inside an async!
// To start executing a future, PASS IT TO AN EXECUTOR!!
//
// You can't put an async in a Trait! Trait is impl on many types!!
// There are buts...with lifetimes!!
//
// There is async-trait package!! (Boxes! -> Less efficient)

fn main() {
    println!("Hello, world!");
    //let g = genums();
    let (ex, sp) = new_executor_and_spawner();

    sp.spawn(async {
        println!("Howdy!");
        TimerFuture::new(Duration::new(2,0)).await;
        print!("Done!");
    });

    // Drop spawner so executor knows it is finished
    // Save resources basically Otherwise, will hang! (recv??)
    drop(sp);

    ex.run();
}
