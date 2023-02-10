use crate::{print, println};
use conquer_once::spin::OnceCell;
use core::pin::Pin;
use core::task::{Context, Poll};
use crossbeam_queue::ArrayQueue;
use futures_util::task::AtomicWaker;
use futures_util::{Stream, StreamExt};
use pc_keyboard::{layouts, DecodedKey, HandleControl, Keyboard, ScancodeSet1};

// コンパイル時にヒープ割り当てできないので、OnceCellで静的な値の安全な一回限りの初期化をする
static SCANCODE_QUEUE: OnceCell<ArrayQueue<u8>> = OnceCell::uninit();

// キューが初期化されていなかったら警告を出す
// この関数は割り込みハンドラから呼び出され、キューの初期化は割り込みハンドラから行うべきではない
// main.rsからも呼び出し可能であってはいけないので、pub(crate)にしている
pub(crate) fn add_scancode(scancode: u8) {
    println!("[add_scancode]add scancode!!");
    if let Ok(queue) = SCANCODE_QUEUE.try_get() {
        if let Err(_) = queue.push(scancode) {
            println!("WARNING: SCANCODE_QUEUEが満タンです")
        } else {
            println!("[add_scancode]Wakeup!!");
            WAKER.wake(); // ここを追加
        }
    } else {
        println!("WARNING: SCANCODE_QUEUEが初期化されていません")
    }
}

// _privateフィールドはモジュールの外部から構造体を構築できないようにするため追加
pub struct ScancodeStream {
    _private: (),
}

impl ScancodeStream {
    pub fn new() -> Self {
        SCANCODE_QUEUE
            .try_init_once(|| ArrayQueue::new(100))
            .expect("ScancodeStream::newは一度しか呼び出せません");
        ScancodeStream { _private: () }
    }
}

impl Stream for ScancodeStream {
    type Item = u8;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        println!("[poll_next]poll_next!!");
        // キューへの参照取得(newで初期化しているので、失敗しないはず)
        let queue = SCANCODE_QUEUE.try_get().expect("初期化されてません");

        // キューが空ではなかったらWAKERを登録しなくていいので早期リターン
        if let Ok(scancode) = queue.pop() {
            println!("[poll_next][1st]queue is not empty!!");
            return Poll::Ready(Some(scancode));
        }
        // キューが空かもしれないのでWAKER登録
        println!("[poll_next]waker register!!");
        WAKER.register(&cx.waker());
        match queue.pop() {
            Ok(scancode) => {
                // 通知が不要なのでWAKERを消す
                println!("[poll_next][2nd]queue is not empty!!");
                WAKER.take();
                Poll::Ready(Some(scancode))
            }
            Err(crossbeam_queue::PopError) => {
                println!("[poll_next][2nd]queue is empty!!");
                Poll::Pending
            } // queueが空の場合
        }
    }
}

// poll_nextでctxに含まれるwakerをここに格納する
// add_scancodeではこのWAKERを呼び出す
static WAKER: AtomicWaker = AtomicWaker::new();

pub async fn print_keypresses() {
    let mut scancodes = ScancodeStream::new();
    let mut keyboard = Keyboard::new(layouts::Us104Key, ScancodeSet1, HandleControl::Ignore);

    println!("[print_keypresses]start wait!!");
    while let Some(scancode) = scancodes.next().await {
        println!("[print_keypresses]key pressed!!");
        if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
            if let Some(key) = keyboard.process_keyevent(key_event) {
                match key {
                    DecodedKey::Unicode(character) => print!("{}", character),
                    DecodedKey::RawKey(key) => print!("{:?}", key),
                }
            }
        }
    }
}
