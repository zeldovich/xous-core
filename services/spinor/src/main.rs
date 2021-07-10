#![cfg_attr(target_os = "none", no_std)]
#![cfg_attr(target_os = "none", no_main)]

mod api;

use num_traits::FromPrimitive;

use log::info;


#[cfg(target_os = "none")]
mod implementation {
    use utralib::generated::*;
    // use crate::api::*;
    use log::info;
    use susres::{RegManager, RegOrField, SuspendResume};

    pub struct Spinor {
        csr: utralib::CSR<u32>,
        fifo: xous::MemoryRange,
        susres_manager: RegManager::<{utra::audio::AUDIO_NUMREGS}>,
    }

    impl Spinor {
        pub fn new() -> Spinor {
            let csr = xous::syscall::map_memory(
                xous::MemoryAddress::new(utra::audio::HW_AUDIO_BASE),
                None,
                4096,
                xous::MemoryFlags::R | xous::MemoryFlags::W,
            )
            .expect("couldn't map Audio CSR range");
            let fifo = xous::syscall::map_memory(
                xous::MemoryAddress::new(utralib::HW_AUDIO_MEM),
                None,
                utralib::HW_AUDIO_MEM_LEN,
                xous::MemoryFlags::R | xous::MemoryFlags::W,
            )
            .expect("couldn't map Audio CSR range");

            let mut spinor = Spinor {
                csr: CSR::new(csr.as_mut_ptr() as *mut u32),
                susres_manager: RegManager::new(csr.as_mut_ptr() as *mut u32),
                fifo,
            };

            spinor
        }

        pub fn suspend(&mut self) {
            self.susres_manager.suspend();
        }
        pub fn resume(&mut self) {
            self.susres_manager.resume();
        }
    }
}

// a stub to try to avoid breaking hosted mode for as long as possible.
#[cfg(not(target_os = "none"))]
mod implementation {
    use log::info;

    pub struct Spinor {
    }

    impl Spinor {
        pub fn new() -> Spinor {
            Spinor {
            }
        }
        pub fn suspend(&self) {
        }
        pub fn resume(&self) {
        }
    }
}


#[xous::xous_main]
fn xmain() -> ! {
    use crate::implementation::Spinor;

    log_server::init_wait().unwrap();
    log::set_max_level(log::LevelFilter::Info);
    info!("my PID is {}", xous::process::id());

    let xns = xous_names::XousNames::new().unwrap();
    /*
        Very important to track who has access to the SPINOR server, and limit access. Access to this server is essential for persistent rootkits.
        Here is the list of servers allowed to access, and why:
          - shellchat (for testing ONLY, remove once done)
          - PDDB (not yet written)
          - keystore (not yet written)
    */
    let spinor_sid = xns.register_name(api::SERVER_NAME_SPINOR, Some(1)).expect("can't register server");
    log::trace!("registered with NS -- {:?}", spinor_sid);

    let mut spinor = Spinor::new();

    log::trace!("ready to accept requests");

    // register a suspend/resume listener
    let sr_cid = xous::connect(spinor_sid).expect("couldn't create suspend callback connection");
    let mut susres = susres::Susres::new(&xns, api::Opcode::SuspendResume as u32, sr_cid).expect("couldn't create suspend/resume object");

    loop {
        let msg = xous::receive_message(spinor_sid).unwrap();
        match FromPrimitive::from_usize(msg.body.id()) {
            Some(api::Opcode::SuspendResume) => xous::msg_scalar_unpack!(msg, token, _, _, _, {
                spinor.suspend();
                susres.suspend_until_resume(token).expect("couldn't execute suspend/resume");
                spinor.resume();
            }),
            None => {
                log::error!("couldn't convert opcode");
                break
            }
        }
    }
    // clean up our program
    log::trace!("main loop exit, destroying servers");
    xns.unregister_server(spinor_sid).unwrap();
    xous::destroy_server(spinor_sid).unwrap();
    log::trace!("quitting");
    xous::terminate_process(0)
}