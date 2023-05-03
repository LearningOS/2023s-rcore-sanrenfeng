//! Process management syscalls
use crate::{
    config::MAX_SYSCALL_NUM,
    task::{
        current_task_mmap,change_program_brk, exit_current_and_run_next, suspend_current_and_run_next, TaskStatus, current_user_token, current_task_time, current_task_sys_time, current_task_munmap,
    }, timer::{get_time_us, get_time_ms},
    mm::{translate_va_t, VirtAddr, MapPermission},
};

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// Task information
#[allow(dead_code)]
pub struct TaskInfo {
    /// Task status in it's life cycle
    status: TaskStatus,
    /// The numbers of syscall called by task
    syscall_times: [u32; MAX_SYSCALL_NUM],
    /// Total running time of task
    time: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(_exit_code: i32) -> ! {
    trace!("kernel: sys_exit");
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(_ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    let us = get_time_us();
    let ts = translate_va_t(current_user_token(), (_ts as usize).into()) as &'static mut TimeVal; 
    *ts = TimeVal{sec: us / 1_000_000,
    usec :  us % 1_000_000,
    };
    _tz as isize
}

/// YOUR JOB: Finish sys_task_info to pass testcases
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TaskInfo`] is splitted by two pages ?
pub fn sys_task_info(_ti: *mut TaskInfo) -> isize {
    trace!("kernel: sys_task_info NOT IMPLEMENTED YET!");
    let ti = translate_va_t(current_user_token(), (_ti as usize).into() )as &'static mut TaskInfo;
    *ti = TaskInfo{
     status: TaskStatus::Running,
     syscall_times: current_task_sys_time() , 
     /// Total running time of task
     time: get_time_ms() - current_task_time(),    
    }; 
    0
}

// YOUR JOB: Implement mmap.
pub fn sys_mmap(_start: usize, _len: usize, _port: usize) -> isize {
        let start : VirtAddr = _start.into();
        let end : VirtAddr = (_start + _len).into();
        if start.page_offset()!=0{
            return -1;
        }
        if (_port & !0x7 !=0 ) || (_port & 0x7 ==0) {
            return -1;
        }
        // 第 0 位表示是否可读，第 1 位表示是否可写，第 2 位表示是否可执行。其他位无效且必须为 0
        let mut flag = MapPermission::U ;
        if _port & 1 == 1{
            flag = flag | MapPermission::R;
        } 
        if _port>>1 & 1 == 1{
            flag = flag | MapPermission::W;
        }
        if _port>>2 & 1 == 1{
            flag = flag | MapPermission::X;
        }
        //info!("{:#?} =>>=>> {:#?}",start,_len,);

        current_task_mmap(start, end, flag) 

}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(_start: usize, _len: usize) -> isize {
       trace!("kernel: sys_munmap NOT IMPLEMENTED YET!");
        let start : VirtAddr = _start.into();
        let end : VirtAddr = (_start + _len).into();
        if start.page_offset()!=0{
            return -1;
        }
        
        info!("sys_munmap{:#?} =>>=>> {:#?}",start,_len,);

        current_task_munmap(start, end)
}
/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel: sys_sbrk");
    if let Some(old_brk) = change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}
