use crate::definitions::Real;
use crate::currency::Currency;
use crate::instrument::InstrumentTrait;
//
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Cash {
    currency: Currency,
    name: String,
    code: String,
}

impl Cash {
    pub fn new(currency: Currency) -> Self {
        Cash { 
            currency,
            name: currency.as_str().to_string(),
            code: currency.as_str().to_string(),
        }
    }
}

impl InstrumentTrait for Cash {
    fn get_name(&self) -> &String {
        &self.name
    }

    fn get_unit_notional(&self) -> Real {
        1.0
    }
    fn get_code(&self) -> &String {
        &self.name
    }

    fn get_currency(&self) -> &Currency {
        &self.currency
    }

    fn get_type_name(&self) -> &'static str {
        "Cash"
    }
}