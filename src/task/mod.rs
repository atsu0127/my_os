use alloc::boxed::Box;
use core::future::Future;
use core::pin::Pin;
use core::sync::atomic::{AtomicU64, Ordering};
use core::task::{Context, Poll};

pub mod executor;
pub mod keyboard;
pub mod simple_executor;

// OrdをつけてるのはBTreeMapのキーとして利用したいから
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct TaskId(u64);

impl TaskId {
    fn new() -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);
        // 一回のアトミックな操作で値を増やし前の値を返す
        TaskId(NEXT_ID.fetch_add(1, Ordering::Relaxed))
    }
}

// Task構造体はピン留めされて、ヒープに割り当てられ、空の型を出力する動的ディスパッチされるfutureのラッパー
pub struct Task {
    id: TaskId, // new
    // futureが()を返すことを要求する => 結果はなく副作用のためだけに実行される
    // dynキーワードでBoxにtrait objを格納することを示す、こうすることでTask型に異なる型のfutureを格納できる
    // Pin<Box>型によって構造体をheapに配置し、その値への&mutな参照を防ぐことで移動できないようにする
    future: Pin<Box<dyn Future<Output = ()>>>,
}

impl Task {
    // 任意のfutureを受け取り、Box::pinでメモリのピン留めする
    // 'staticライフタイムは返されたTaskが任意の時間だけ生き残るので、futureもその時間だけ有効である必要があるため必要
    pub fn new(future: impl Future<Output = ()> + 'static) -> Task {
        Task {
            id: TaskId::new(), // new
            future: Box::pin(future),
        }
    }

    // Future traitのpollメソッドはPin<&mut T>型で呼び出されることを想定しているので
    // Pin::as_mutを使ってself.futureをPin<&mut T>型に変換し、pollを呼び出す
    // Task::pollはexecutorのみから呼び出されるので、privateにしている
    fn poll(&mut self, context: &mut Context) -> Poll<()> {
        self.future.as_mut().poll(context)
    }
}
