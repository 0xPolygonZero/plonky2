use hashbrown::HashSet;
use lazy_static::lazy_static;
use spin::{Mutex, MutexGuard};

lazy_static! {
    static ref LAST_CIR_RAND_DATA: Mutex<CirRandData> = Mutex::new(CirRandData {
        random_values: HashSet::new(),
    });
}

pub struct CirRandData {
    pub random_values: HashSet<u64>,
}

pub fn set_last_cir_rand_data(cir_data: CirRandData) {
    let mut data = LAST_CIR_RAND_DATA.lock();
    *data = cir_data;
}

pub fn get_last_cir_data() -> MutexGuard<'static, CirRandData> {
    LAST_CIR_RAND_DATA.lock()
}
