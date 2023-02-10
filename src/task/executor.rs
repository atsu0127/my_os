use crate::println;
use crate::task::{Task, TaskId};
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::task::Wake;
use core::task::{Context, Poll, Waker};
use crossbeam_queue::ArrayQueue;

pub struct Executor {
    // 実際にTaskを格納しているBtreeMap
    tasks: BTreeMap<TaskId, Task>,
    // タスクIDを格納するqueue
    // Arcを使っているのはtask_queueがexecutorとwakerで共有されるから
    // wakerが起こされたTaskIdをqueueに入れて、executorはそれを受けてTaskを実行する
    task_queue: Arc<ArrayQueue<TaskId>>,
    // Taskが作成された後にそのタスクのWakerをcacheする
    // - 同じタスクのwakeupに対してはwakerを使いまわしたい
    // - 参照カウントされるwakerが割り込みハンドラ内で解放されないようにするため
    waker_cache: BTreeMap<TaskId, Waker>,
}

impl Executor {
    pub fn new() -> Self {
        Executor {
            tasks: BTreeMap::new(),
            task_queue: Arc::new(ArrayQueue::new(100)),
            waker_cache: BTreeMap::new(),
        }
    }

    pub fn spawn(&mut self, task: Task) {
        println!("[Executor::spawn] task spawned: {}", &task.id.0);
        let task_id = task.id;
        if self.tasks.insert(task.id, task).is_some() {
            panic!("同じTaskIdがすでにtasks内にあります");
        }
        self.task_queue
            .push(task_id)
            .expect("queueへのTaskIdの追加に失敗");
    }

    fn run_ready_tasks(&mut self) {
        // self.task_queueをclosureの中でアクセスするがその際、selfを完全に借用してしまうので分配
        let Self {
            tasks,
            task_queue,
            waker_cache,
        } = self;

        while let Ok(task_id) = task_queue.pop() {
            println!("[Executor::run_ready_tasks] popped: {}", &task_id.0);
            // popされたTaskIdに対して、Taskを取得
            let task = match tasks.get_mut(&task_id) {
                Some(task) => task,
                None => continue,
            };
            // waker_cacheからwakerの取得
            let waker = waker_cache
                .entry(task_id)
                .or_insert_with(|| TaskWaker::new(task_id, task_queue.clone()));
            println!("[Executor::run_ready_tasks] waker got");

            let mut context = Context::from_waker(waker);
            match task.poll(&mut context) {
                // TaskがReadyを返したら完了しているので、TaskIdに紐づくものを消す
                Poll::Ready(()) => {
                    println!("[Executor::run_ready_tasks] polling Ready");
                    tasks.remove(&task_id);
                    waker_cache.remove(&task_id);
                }
                Poll::Pending => {
                    println!("[Executor::run_ready_tasks] polling Pending");
                }
            }
        }
    }

    pub fn run(&mut self) -> ! {
        loop {
            self.run_ready_tasks();
            self.sleep_if_idle();
        }
    }

    fn sleep_if_idle(&self) {
        use x86_64::instructions::interrupts::{self, enable_and_hlt};

        // 一旦割り込みを無効にして
        interrupts::disable();
        if self.task_queue.is_empty() {
            // queueが空なら割り込み有効にしてhlt
            enable_and_hlt();
        } else {
            // queueが空じゃないなら割り込み有効にするだけ
            interrupts::enable();
        }
    }
}

// 起こされたTaskIdをtask_queueにpushするためのWaker
struct TaskWaker {
    task_id: TaskId,
    task_queue: Arc<ArrayQueue<TaskId>>,
}

impl TaskWaker {
    // 渡されたtask_idとtask_queueを使ってWakerを作る
    // Arcでラップしている
    fn new(task_id: TaskId, task_queue: Arc<ArrayQueue<TaskId>>) -> Waker {
        Waker::from(Arc::new(TaskWaker {
            task_id,
            task_queue,
        }))
    }

    fn wake_task(&self) {
        println!("[TaskWaker::wake_task] wake task");
        self.task_queue
            .push(self.task_id)
            .expect("task_queueが満タンです")
    }
}

// Wake traitを実装しFromでWakerにする
// 以下の実装を呼び出すためには、TaskWakerをArcでラップする必要がある
impl Wake for TaskWaker {
    // Arcの所有権を取るため、参照カウントが上がるかもしれない
    fn wake(self: Arc<Self>) {
        self.wake_task();
    }

    // Arcへの参照しか取らない
    fn wake_by_ref(self: &Arc<Self>) {
        self.wake_task();
    }
}
