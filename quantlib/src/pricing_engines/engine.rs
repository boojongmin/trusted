use crate::data::observable::Observable;
use crate::instruments::instrument_info::InstrumentInfo;
use crate::instruments::stock_futures::StockFutures;
use crate::parameters::zero_curve_code::ZeroCurveCode;
use crate::parameters::discrete_ratio_dividend::DiscreteRatioDividend;
use crate::parameters::zero_curve::{self, ZeroCurve};
use crate::pricing_engines::calculation_configuration::CalculationConfiguration;
use crate::evaluation_date::EvaluationDate;
use crate::instrument::{Instrument, Instruments};
use crate::pricing_engines::calculation_result::CalculationResult;
use crate::definitions::{Real, FX};
use crate::assets::stock::{self, Stock};
use crate::data::vector_data::VectorData;
use crate::data::value_data::ValueData;
use crate::pricing_engines::pricer::{Pricer, PricerTrait};
use crate::pricing_engines::stock_futures_pricer::StockFuturesPricer;
use crate::pricing_engines::match_parameter::MatchPrameter;
use core::borrow;
use std::collections::HashMap;
use std::rc::Rc;
use std::cell::RefCell;
use serde_json::Value;
use time::{OffsetDateTime, Duration};
use anyhow::{Context, Result};
use crate::utils::myerror::{MyError, VectorDisplay};

/// Engine typically handles a bunch of instruments and calculate the pricing of the instruments.
/// Therefore, the result of calculations is a hashmap with the key being the code of the instrument
/// Engine is a struct that holds the calculation results of the instruments
pub struct Engine<'a> {
    calculation_result: HashMap<&'a str, CalculationResult<'a>>,
    calculation_configuration: CalculationConfiguration,
    stock_data: HashMap<&'a str, ValueData>,
    fx_data: HashMap<&'a str, ValueData>,
    curve_data: HashMap<&'a str, VectorData>,
    dividend_data: HashMap<&'a str, VectorData>,
    //
    evaluation_date: Rc<RefCell<EvaluationDate>>,
    fxs: HashMap<&'a str, Rc<RefCell<Real>>>,
    stocks: HashMap<&'a str, Rc<RefCell<Stock>>>,
    zero_curves: HashMap<&'a str, Rc<RefCell<ZeroCurve>>>,
    dividends: HashMap<&'a str, Rc<RefCell<DiscreteRatioDividend>>>,
    // instruments
    instruments: Instruments<'a>, // all instruments
    pricers: HashMap<&'a str, Pricer>, // pricers for each instrument
    // selected instuments for calculation,
    // e.g., if we calcualte a delta of a single stock, we do not need calculate all instruments
    instruments_in_action: Vec<&'a Instrument<'a>>, 
    //
    match_parameter: MatchPrameter<'a>,
}

impl<'a> Engine<'a> {
    pub fn new (
        calculation_configuration: CalculationConfiguration,
        evaluation_date: EvaluationDate,
        //
        fx_data: HashMap<&'a str, ValueData>,
        stock_data: HashMap<&'a str, ValueData>,
        curve_data: HashMap<&'a str, VectorData>,
        dividend_data: HashMap<&'a str, VectorData>,
        //
        instruments: Instruments<'a>,
        match_parameter: MatchPrameter<'a>,
    ) -> Engine<'a> {
        let evaluation_date = Rc::new(RefCell::new(
            evaluation_date
        ));

        let zero_curves = HashMap::new();
        for (key, data) in curve_data.iter() {
            let zero_curve = Rc::new(RefCell::new(
                ZeroCurve::new(
                    evaluation_date.clone(),
                    data,
                    ZeroCurveCode::from_str(key).unwrap(),
                    key.to_string(),
                ).expect("failed to create zero curve")
            ));
            zero_curves.insert(*key, zero_curve.clone());
            data.add_observer(zero_curve.clone());
        }

        let dividends = HashMap::new();
        for (key, data) in dividend_data.iter() {
            let spot = stock_data.get(key)
                .expect("Failed to find stock data matching the dividend data")
                .get_value();

            let dividend = Rc::new(RefCell::new(
                DiscreteRatioDividend::new(
                    evaluation_date.clone(),
                    data,
                    spot,
                    key.to_string(),
                ).expect("failed to create discrete ratio dividend")
            ));
            dividends.insert(*key, dividend.clone());
            data.add_observer(dividend.clone());
        }
        // making fx Rc -> RefCell for pricing
        let fxs: HashMap<&str, Rc<RefCell<Real>>> = HashMap::new();
        fx_data
            .iter()
            .map(|(key, data)| {
                let rc = Rc::new(RefCell::new(data.get_value()));
                fxs.insert(*key, rc)
            });

        // making stock Rc -> RefCell for pricing
        let stocks = HashMap::new();
        for (key, data) in stock_data.iter() {
            let div = match dividends.get(key) {
                Some(div) => Some(div.clone()),
                None => None,
            };

            let rc = Rc::new(RefCell::new(
                Stock::new(
                    data.get_value(),
                    data.get_market_datetime().clone(),
                    div,
                    data.get_currency().clone(),  
                    data.get_name().clone(),
                    key.to_string(),
                )
            ));
            stocks.insert(*key, rc);
            }
        
        Engine {
            calculation_result: HashMap::new(),
            calculation_configuration,
            //
            stock_data,
            fx_data,
            curve_data,
            dividend_data,
            //
            evaluation_date,
            fxs,
            stocks,
            zero_curves,
            dividends,
            //
            instruments: instruments,
            instruments_in_action: vec![],
            pricers: HashMap::new(),
            match_parameter: match_parameter,
        }
    }
    
    // initialize CalculationResult for each instrument
    pub fn initialize(&mut self, instrument_vec: Vec<&'a Instrument<'a>>) -> Result<()> {
        self.initialize_instruments(instrument_vec)
            .with_context(|| format!(
                "(Engine::initialize) Failed to initialize instruments\n\
                occuring at {file}:{line}",
                file = file!(),
                line = line!(),
            ))?;

        self.initialize_pricers()
            .with_context(|| format!(
                "(Engine::initialize) Failed to initialize pricers\n\
                occuring at {file}:{line}",
                file = file!(),
                line = line!(),
            ))?;

        Ok(())
    }

    pub fn initialize_instruments(mut self, instrument_vec: Vec<&'a Instrument<'a>>) -> Result<(), MyError> {
        if instrument_vec.is_empty() {
            return Err(
                MyError::EmptyVectorError {
                    file: file!().to_string(),
                    line: line!(),
                    other_info: "(Engine::initialize_instruments) instruments are empty".to_string(),
                }
            );
        }

        self.instruments = Instruments::new(instrument_vec);
        self.instruments_in_action = self.instruments.get_instruments().clone();
    
        for instrument in self.instruments.iter() {
            let inst = instrument.as_trait();
            let code = inst.get_code();
            let inst_type = inst.get_type_name();
            let instrument_information = InstrumentInfo::new(
                inst.get_name(),
                code,
                inst_type,
                inst.get_currency().clone(),
                inst.get_unit_notional(),
                inst.get_maturity().clone(),
            );

            let init_res = CalculationResult::new(
                instrument_information,
                self.evaluation_date.borrow().get_date_clone(),
            );

            self.calculation_result.insert(
                inst.get_code(),
                init_res
            );
        }
        Ok(())
    }

    pub fn initialize_pricers(mut self) -> Result<(), MyError> {
        let inst_vec = self.instruments.get_instruments();
        for inst in inst_vec.iter() {
            let inst_type = inst.as_trait().get_type_name();
            let inst_name = inst.as_trait().get_name();
            let inst_code = inst.as_trait().get_code();
            let inst_curr = inst.as_trait().get_currency();
            let undertlying_codes = inst.as_trait().get_underlying_codes();

            let pricer = match inst {
                Instrument::StockFutures(instrument) => {
                    let stock = self.stocks.get(undertlying_codes[0]).unwrap().clone();
                    let collatral_curve_name = self.match_parameter.get_collateral_curve_name(inst);
                    let borrowing_curve_name = self.match_parameter.get_borrowing_curve_name(inst);
                    let core = StockFuturesPricer::new(
                        stock,
                        self.zero_curves.get(collatral_curve_name).unwrap().clone(),
                        self.zero_curves.get(borrowing_curve_name).unwrap().clone(),
                        self.evaluation_date.clone(),
                    );
                    Pricer::StockFuturesPricer(Box::new(core))
                },
                _ => {
                    return Err(
                        MyError::BaseError {
                            file: file!().to_string(), 
                            line: line!(), 
                            contents: format!(
                                "Not implemented yet (type = {}, name =  {})", 
                                inst_type,
                                inst_name
                            )
                        }
                    );
                }
            };
            self.pricers.insert(inst_code, pricer);
        }
        Ok(())
    }

    pub fn get_npvs(&self) -> Result<HashMap<&str, Real>> {
        let mut npvs = HashMap::new();
        for inst in self.instruments_in_action {
            let pricer = self.pricers.get(inst.as_trait().get_code())
                .with_context(|| format!(
                    "(Engine::get_npv) Failed to get pricer for {name}\n\
                    occuring at {file}:{line}",
                    name = inst.as_trait().get_name(),
                    file = file!(),
                    line = line!(),
                ))?;
            let npv = pricer.as_trait().npv(inst)
                .expect("calculation failed");
            npvs.insert(inst.as_trait().get_code(), npv);
        }
        Ok(npvs)
    }

    pub fn set_npvs(&mut self) -> Result<()> {
        let npv = self.get_npvs()
            .with_context(|| format!(
                "(Engine::set_npv) Failed to get npv\n\
                occuring at {file}:{line}",
                file = file!(),
                line = line!(),
            ))?;
        
        for (code, result) in self.calculation_result.iter_mut() {
            result.set_npv(
                npv.get(code)
                    .expect("npv is not set").clone()
                );
        }
        Ok(())
    }

    pub fn set_fx_exposures(&mut self) -> Result<()> {
        let mut fx_exposures = HashMap::new();
        for inst in self.instruments_in_action {
            let pricer = self.pricers.get(inst.as_trait().get_code())
                .with_context(|| format!(
                    "(Engine::set_fx_exposure) Failed to get pricer for {name}\n\
                    occuring at {file}:{line}",
                    name = inst.as_trait().get_name(),
                    file = file!(),
                    line = line!(),
                ))?;
            let fx_exposure = pricer.as_trait().fx_exposure(inst)
                .expect("calculation failed");
            fx_exposures.insert(inst.as_trait().get_code(), fx_exposure);
        }
        for (code, result) in self.calculation_result.iter_mut() {
            result.set_fx_exposure(
                fx_exposures.get(code)
                    .expect("fx_exposure is not set").clone()
                );
        }
        Ok(())
    }

    /// Set the value of the instruments which means npv * unit_notional
    pub fn set_values(&mut self) {
        for (_code, result) in self.calculation_result.iter_mut() {
            result.set_value();
        }
    }

    
    pub fn set_delta(&mut self) -> Result<()> {
        let all_underlying_codes = self.instruments.get_underlying_codes();
        let delta_bump_ratio = self.calculation_configuration.get_delta_bump_ratio();

        for und_code in all_underlying_codes.iter() {
            self.instruments_in_action = self.instruments
                .instruments_with_underlying(und_code);
            let stock = self.stocks.get(und_code)
                .expect("there is no stock")
                .clone();
        }
            
        Ok(())
    }
    
    pub fn set_coupons(&mut self) {
        for inst in self.instruments_in_action {
            let start_date = self.evaluation_date.borrow().get_date_clone();
            let theta_day = self.calculation_configuration.get_theta_day();
            let end_date = start_date + Duration::days(theta_day as i64);
            let coupons = self.pricers.get(inst.as_trait().get_code())
                .expect("pricer is not set")
                .as_trait()
                .coupons(inst, &start_date, &end_date)
                .expect("calculation failed");
            self.calculation_result.get_mut(inst.as_trait().get_code())
                .expect("result is not set")
                .set_cashflow_inbetween(coupons);
        }
    }

    pub fn calculate(&mut self) {
        self.set_npvs();
        self.set_values();
        self.set_fx_exposures();
        self.set_coupons();
        // if delta is true, calculate delta
        if self.calculation_configuration.get_delta_calculation() {
            self.set_delta();
        }

    }
    pub fn get_calculation_result(&self) -> &HashMap<&str, CalculationResult> {
        &self.calculation_result
    }

    pub fn get_calculation_result_clone(&self) -> HashMap<&str, CalculationResult> {
        self.calculation_result.clone()
    }
}