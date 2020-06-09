use core::ptr;
use core::option::Option;
use core::convert::TryFrom;

use crate::consts::{TRAMPOLINE, TRAPFRAME, PGSIZE};
use crate::register::{satp, sepc};
use crate::spinlock::SpinLock;
use crate::mm::{Box, PageTable, VirtAddr, PhysAddr, PteFlag};
use crate::trap::user_trap;

use super::PROC_MANAGER;
use super::{Context, TrapFrame, fork_ret, cpu_id};

#[derive(Eq, PartialEq, Debug)]
pub enum ProcState { UNUSED, SLEEPING, RUNNABLE, RUNNING, ZOMBIE }

pub struct Proc {
    pub lock: SpinLock<()>,

    // p->lock must be held when using these:
    pub state: ProcState,
    pub killed: bool,
    pub pid: usize,

    // lock need not be held, or
    // lock already be held
    kstack: usize,
    sz: usize,
    pagetable: Option<Box<PageTable>>,
    tf: *mut TrapFrame,
    context: Context,
    name: [u8; 16],
}

impl Proc {
    pub const fn new() -> Self {
        Self {
            lock: SpinLock::new((), "proc"),
            state: ProcState::UNUSED,
            killed: false,
            pid: 0,
            kstack: 0,
            sz: 0,
            pagetable: None,
            tf: ptr::null_mut(),
            context: Context::new(),
            name: [0; 16],
        }
    }

    pub fn set_kstack(&mut self, kstack: usize) {
        self.kstack = kstack;
    }

    /// Allocate a new user pagetable for itself
    /// and map trampoline code and trapframe
    pub fn proc_pagetable(&mut self) {
        extern "C" {
            fn trampoline();
        }

        let mut pagetable = PageTable::uvm_create();
        pagetable.map_pages(VirtAddr::from(TRAMPOLINE), PGSIZE,
            PhysAddr::try_from(trampoline as usize).unwrap(), PteFlag::R | PteFlag::X)
            .expect("user proc table mapping trampoline");
        pagetable.map_pages(VirtAddr::from(TRAPFRAME), PGSIZE,
            PhysAddr::try_from(self.tf as usize).unwrap(), PteFlag::R | PteFlag::W)
            .expect("user proc table mapping trapframe");

        self.pagetable = Some(pagetable);
    }

    pub fn set_tf(&mut self, tf: *mut TrapFrame) {
        self.tf = tf;
    }

    /// Init the context of the process after it is created
    /// Set its return address to fork_ret,
    /// which start to return to user space.
    pub fn init_context(&mut self) {
        self.context.clear();
        self.context.set_ra(fork_ret as *const () as usize);
        self.context.set_sp(self.kstack + PGSIZE);
    }

    /// Return the process's mutable reference of context
    pub fn get_context_mut(&mut self) -> &mut Context {
        &mut self.context
    }

    /// Called by ProcManager's user_init,
    /// Only be called once for the first user process
    /// TODO - copy user code and sth else
    pub fn user_init(&mut self) {
        // map initcode in user pagetable
        self.pagetable.as_mut().unwrap().uvm_init(&INITCODE);
        self.sz = PGSIZE;

        // prepare return pc and stack pointer
        let tf: &mut TrapFrame = unsafe {&mut *self.tf};
        tf.epc = 0;
        tf.set_sp(PGSIZE);

        let init_name = b"initcode\0";
        unsafe {
            ptr::copy_nonoverlapping(init_name.as_ptr(),
                self.name.as_mut_ptr(), init_name.len());
        }
        // TODO - p->cwd = namei("/");

        self.state = ProcState::RUNNABLE;
    }

    // Prepare things before sret to user space
    pub fn user_ret_prepare(&mut self) -> usize {
        let tf: &mut TrapFrame = unsafe {&mut *self.tf};
        tf.kernel_satp = satp::read();
        // current kernel stack's content is cleaned
        // after returning to the kernel space
        tf.kernel_sp = self.kstack + PGSIZE;
        tf.kernel_trap = user_trap as usize;
        tf.kernel_hartid = unsafe {cpu_id()};

        // restore the user pc previously stored in sepc
        sepc::write(tf.epc);

        self.pagetable.as_ref().unwrap().as_satp()
    }

    /// Exit the current process. No return.
    /// LTODO - An exited process remains in the zombie state
    ///     until its parent calls wait()
    pub fn exit(&mut self, status: isize) {
        if unsafe {PROC_MANAGER.is_init_proc(&self)} {
            panic!("init_proc exiting");
        }

        panic!("exit: TODO, status={}", status);
    }
}

/// from xv6-riscv:
/// first user program that calls exec("/init")
static INITCODE: [u8; 51] = [
    0x17, 0x05, 0x00, 0x00, 0x13, 0x05, 0x05, 0x02,
    0x97, 0x05, 0x00, 0x00, 0x93, 0x85, 0x05, 0x02,
    0x9d, 0x48, 0x73, 0x00, 0x00, 0x00, 0x89, 0x48,
    0x73, 0x00, 0x00, 0x00, 0xef, 0xf0, 0xbf, 0xff,
    0x2f, 0x69, 0x6e, 0x69, 0x74, 0x00, 0x00, 0x01,
    0x20, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00
];
