pub trait Instrument {
    fn price(&self) -> f64;
    fn value(&self) -> f64;
    fn npv(&self) -> f64;
    fn clean_price(&self) -> f64;
    fn dirty_price(&self) -> f64;
    fn accrued_amount(&self) -> f64;
    fn accrued_days(&self) -> f64;
    fn settlement_value(&self) -> f64;
    fn settlement_days(&self) -> f64;
    fn settlement_amount(&self) -> f64;
    fn settlement_date(&self) -> f64;
    fn maturity_date(&self) -> f64;
    fn is_expired(&self) -> bool;
    fn notional(&self) -> f64;
    fn notional_at(&self, date: f64) -> f64;
    fn is_expired_at(&self, date: f64) -> bool;
    fn previous_coupon_date(&self, date: f64) -> f64;
    fn next_coupon_date(&self, date: f64) -> f64;
    fn previous_cashflow_date(&self, date: f64) -> f64;
    fn next_cashflow_date(&self, date: f64) -> f64;
    fn previous_cashflow_amount(&self, date: f64) -> f64;
    fn next_cashflow_amount(&self, date: f64) -> f64;
    fn previous_cashflow_date_amount(&self, date: f64) -> (f64, f64);
    fn next_cashflow_date_amount(&self, date: f64) -> (f64, f64);
    fn previous_cashflow_date_amount_at(&self, date: f64, settlement_date: f64) -> (f64, f64);
    fn next_cashflow_date_amount_at(&self, date: f64, settlement_date: f64) -> (f64, f64);
    fn previous_coupon_date_amount(&self, date: f64) -> (f64, f64);
    fn next_coupon_date_amount(&self, date: f64) -> (f64, f64);
    fn previous_coupon_date_amount_at(&self, date: f64, settlement_date: f64) -> (f64, f64);
    fn next_coupon_date_amount_at(&self, date: f64, settlement_date: f64) -> (f64, f64);
}