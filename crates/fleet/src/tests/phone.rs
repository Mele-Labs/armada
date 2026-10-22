//! `get_phone` and `set_phone` answer what the switch the composition root
//! handed in says, and refuse where there is none.

use std::sync::{Arc, Mutex};

use api::{Refusal, Settings};
use ipc::{PhoneGateway, SetPhoneGateway};

use super::session_host::rig_placing;
use crate::phone::PhoneSwitch;

#[derive(Default)]
struct Switch {
    on: Mutex<bool>,
}

impl PhoneSwitch for Switch {
    fn state(&self) -> PhoneGateway {
        let on = *self.on.lock().unwrap();
        PhoneGateway { on, port: 8443, listening: on, why: None }
    }

    fn set(&self, on: bool) -> PhoneGateway {
        *self.on.lock().unwrap() = on;
        self.state()
    }
}

#[tokio::test]
async fn the_phone_is_switched_through_what_the_composition_root_handed_in() {
    let rig = rig_placing(None);
    let refused = rig.fleet.get_phone().await;
    assert!(matches!(&refused, Err(Refusal::IllegalMove(why)) if why.code == "fleet.phone_not_served"), "{refused:?}");

    rig.fleet.switching_phone_with(Arc::new(Switch::default()));
    assert!(!rig.fleet.get_phone().await.unwrap().on);
    let on = rig.fleet.set_phone(SetPhoneGateway { on: true }).await.unwrap();
    assert_eq!(on, PhoneGateway { on: true, port: 8443, listening: true, why: None });
    assert!(rig.fleet.get_phone().await.unwrap().listening);
    assert!(!rig.fleet.set_phone(SetPhoneGateway { on: false }).await.unwrap().on);
}
