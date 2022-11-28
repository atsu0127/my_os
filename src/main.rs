#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(my_os::test_runner)]
#![reexport_test_harness_main = "test_main"]

use bootloader::{entry_point, BootInfo};
use core::panic::PanicInfo;
use my_os::println;
use x86_64::structures::paging::{Page, PageTable};
use x86_64::VirtAddr;

entry_point!(kernel_main);

fn kernel_main(boot_info: &'static BootInfo) -> ! {
    use my_os::memory::active_level_4_table;
    use x86_64::VirtAddr;

    println!("Hello World{}", "!");
    my_os::init();

    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset); // offset持ってくる
    let l4_table = unsafe { active_level_4_table(phys_mem_offset) }; // offsetとCR3の内容からl4のtableの仮想アドレス取得

    for (i, entry) in l4_table.iter().enumerate() {
        if !entry.is_unused() {
            println!("L4 Entry {}: {:?}", i, entry);

            let phys = entry.frame().unwrap().start_address(); // l4のtableのエントリ確認
            let virt = phys.as_u64() + boot_info.physical_memory_offset; // エントリに対してoffset付け加える(= 仮想アドレスにする)
            let ptr = VirtAddr::new(virt).as_mut_ptr(); // ptrにする
            let l3_table: &PageTable = unsafe { &*ptr }; // 取得

            // l3テーブルの空でないエントリを出力する
            for (i, entry) in l3_table.iter().enumerate() {
                if !entry.is_unused() {
                    println!("  L3 Entry {}: {:?}", i, entry);
                }
            }
        }
    }

    #[cfg(test)]
    test_main();

    println!("It did not crash!");
    my_os::hlt_loop();
}

#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    my_os::hlt_loop();
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    my_os::test_panic_handler(info)
}
