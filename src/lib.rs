#![no_std]
#![feature(asm)]
#![feature(global_asm)]
#![allow(dead_code)]

global_asm!(include_str!("asm/entry.S"));
global_asm!(include_str!("asm/kernelvec.S"));

mod console;
mod consts;
#[macro_use]
mod printf;
mod proc;
mod register;
mod rmain;
mod spinlock;
mod start;

#[cfg(feature = "unit_test")]
fn test_main_entry() {
    spinlock::tests::smoke();
}
