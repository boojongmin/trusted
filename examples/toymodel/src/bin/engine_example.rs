use quantlib::enums::{
    OptionDailySettlementType,
    OptionType,
    OptionExerciseType,
};
use quantlib::currency::{Currency, FxCode};
use quantlib::instruments::{
    futures::Futures,
    bond::Bond,
    vanilla_option::VanillaOption,
};
use quantlib::instrument::Instrument;
use quantlib::definitions::Real;
use time::{macros::datetime, Duration};
use ndarray::array;
use ndarray::Array1;
//use quantlib::surfacedatasample;
use std::time::Instant;
use std::sync::Arc;
use quantlib::evaluation_date::EvaluationDate;
use quantlib::pricing_engines::calculation_configuration::CalculationConfiguration;
use quantlib::pricing_engines::match_parameter::MatchParameter;
use std::collections::HashMap;
use quantlib::pricing_engines::engine::Engine;
use quantlib::data::value_data::ValueData;
use quantlib::data::vector_data::VectorData;
use quantlib::enums::{IssuerType, CreditRating, RankType};
use quantlib::time::calendars::{southkorea::SouthKorea, southkorea::SouthKoreaType};
use quantlib::time::calendar::Calendar;
use quantlib::time::jointcalendar::JointCalendar;
use quantlib::time::conventions::{BusinessDayConvention, DayCountConvention, PaymentFrequency};
use quantlib::utils::tracing_timer::CustomOffsetTime;
use anyhow::{Result, Context};
use tracing::{info, Level, span};
use tracing_subscriber::fmt;
use tracing_subscriber::prelude::*;
use tracing_appender::{rolling, non_blocking};

fn main() -> Result<()> {
    let start_time = Instant::now();
    // Set up rolling file appender
    let file_appender = rolling::daily("logs", "my_app.log");
    let (non_blocking_appender, _guard) = non_blocking(file_appender);

    // Set up console layer
    let custom_time = CustomOffsetTime::new(9, 0, 0);
    let console_layer = fmt::layer()
        .with_writer(std::io::stdout)
        .with_timer(custom_time.clone());

    let file_layer = fmt::layer()
        .with_writer(non_blocking_appender)
        .with_timer(custom_time);

    // Combine console and file layers into a subscriber
    let subscriber = tracing_subscriber::registry()
        .with(console_layer)
        .with(file_layer);

    tracing::subscriber::set_global_default(subscriber)
        .expect("Setting default subscriber failed");

    // Create a new span with an `info` level
    let main_span = span!(Level::INFO, "main (toymodel)");
    let _enter = main_span.enter();

    let theta_day = 200;
    let spot: Real = 350.0;
    // evaluation date = 2021-01-01 00:00:00 +09:00
    let dt = datetime!(2024-03-13 16:30:00 +09:00);
    let evaluation_date = EvaluationDate::new(dt);

    // make zero curve named "KSD". First make vector data whose values are 0.03 and 0.04
    // then make it as hash map whose key is "KSD"
    let value = array![0.03358, 0.03358];
    let dates = vec![
        datetime!(2025-03-13 00:00:00 +09:00),
        datetime!(2026-03-13 00:00:00 +09:00),
    ];

    let times = None;
    let market_datetime = dt.clone();
    let zero_curve1 = "KSD".to_string();
    let zero_curve_data1 = VectorData::new(
        &value - 0.0005, 
        Some(dates.clone()), 
        times.clone(), 
        Some(market_datetime), 
        Currency::KRW,
        zero_curve1.clone(),
        zero_curve1.clone(),
    ).expect("Failed to create VectorData for KSD");

    let zero_curve2 = "KRWGOV".to_string();
    let zero_curve_data2 = VectorData::new(
        value,
        Some(dates.clone()), 
        times, 
        Some(market_datetime), 
        Currency::KRW,
        zero_curve2.clone(),
        zero_curve2.clone(),
    ).expect("Failed to create VectorData for KRWGOV");

    let funding_curve1 = "Discount(KRW)".to_string();
    let funding_curve_data1 = VectorData::new(
        array![0.04, 0.04],
        Some(dates.clone()),
        None,
        Some(market_datetime),
        Currency::KRW,
        funding_curve1.clone(),
        funding_curve1.clone(),
    ).expect("failed to make a vector data for funding curve");

    // the borrowing fee curve which amounts to 0.005
    let bor_curve_name = "KOSPI2".to_string();
    let borrowing_curve_data = VectorData::new(
        array![0.005, 0.005],
        Some(dates.clone()),
        None,
        Some(market_datetime),
        Currency::KRW,
        bor_curve_name.clone(),
        bor_curve_name.clone(),
    ).expect("failed to make a vector data for borrowing fee");

    //
    // mapping construction
    let mut zero_curve_map = HashMap::new();
    zero_curve_map.insert(zero_curve1, zero_curve_data1);
    zero_curve_map.insert(zero_curve2, zero_curve_data2);
    zero_curve_map.insert("KOSPI2".to_string(), borrowing_curve_data);
    zero_curve_map.insert(funding_curve1.clone(), funding_curve_data1);

    
    let mut equity_vol_map = HashMap::new();
    let equity_surface_map = HashMap::new();

    //let _equity_surface_data = surfacedatasample!(&market_datetime, spot);
    let equity_constant_vol1 = ValueData::new(
        0.2,
        Some(market_datetime),
        Currency::KRW,
        "KOSPI2".to_string(),
        "KOSPI2".to_string(),
    ).expect("failed to make a value data for equity volatility");

    
    //equity_surface_map.insert("KOSPI2".to_string(), equity_surface_data);
    equity_vol_map.insert("KOSPI2".to_string(), equity_constant_vol1);

    let fx_str1 = "USDKRW";
    let fx_code1 = FxCode::from(fx_str1);
    let fx1 = ValueData::new(
        1300.0,
        Some(market_datetime),
        Currency::KRW,
        fx_str1.to_string(),
        fx_str1.to_string(),
    ).expect("failed to make a value data for fx rate");
    let mut fx_data_map = HashMap::new();
    
    fx_data_map.insert(fx_code1, fx1);

    
    // make a vector data for dividend ratio
    let div_name = "KOSPI2".to_string();
    let dividend_data = VectorData::new(
        Array1::from(vec![3.0, 3.0]),
        Some(vec![datetime!(2024-06-01 00:00:00 +09:00), datetime!(2025-01-01 00:00:00 +09:00)]),
        None,
        Some(market_datetime),
        Currency::KRW,
        div_name.clone(),
        div_name.clone(),
    ).expect("failed to make a vector data for dividend ratio");

    let mut dividend_data_map = HashMap::new();
    dividend_data_map.insert("KOSPI2".to_string(), dividend_data.clone());
    
    // make a stock data
    let stock_name = "KOSPI2".to_string();
    let stock_data = ValueData::new(
        spot,
        Some(market_datetime),
        Currency::KRW,
        stock_name.clone(),
        stock_name.clone(),
    ).expect("failed to make a stock data");

    let mut stock_data_map = HashMap::new();
    stock_data_map.insert("KOSPI2".to_string(), stock_data.clone());
    
    // make two stock futures of two maturities with the same other specs
    // then make a Instruments object with the two stock futures
    let stock_futures1 = Futures::new(
        350.0,
        datetime!(2021-01-01 00:00:00 +09:00),
        datetime!(2021-01-11 00:00:00 +09:00),
        datetime!(2024-06-14 00:00:00 +09:00),
        datetime!(2024-06-14 00:00:00 +09:00),
        250_000.0,
        Currency::KRW,
        Currency::KRW,
        "KOSPI2".to_string(),
        "KOSPI2 Fut Mar21".to_string(),
        "165XXX1".to_string(),
    );

    let stock_futures2 = Futures::new(
        350.0,
        datetime!(2021-01-01 00:00:00 +09:00),
        datetime!(2021-01-01 00:00:00 +09:00),
        datetime!(2025-06-14 00:00:00 +09:00),
        datetime!(2025-06-14 00:00:00 +09:00),
        250_000.0,
        Currency::KRW,
        Currency::KRW,
        "KOSPI2".to_string(),
        "KOSPI2 Fut Jun21".to_string(),
        "165XXX2".to_string(),
    );

    let issuedate = datetime!(2020-01-01 16:30:00 +09:00);
    let maturity = issuedate + Duration::days(365 * 6);
    let issuer_name = "Korea Gov";
    let bond_name = "Virtual KTB";
    let bond_code = "KRxxxxxxxxxx";
    let sk = Calendar::SouthKorea(SouthKorea::new(SouthKoreaType::Settlement));
    let calendar = JointCalendar::new(vec![sk])?;

    let bond_currency = Currency::KRW;
    let issuer_type = IssuerType::Government;
    let credit_rating = CreditRating::None;

    let bond = Bond::new_from_conventions(
        issuer_type,
        credit_rating,     
        issuer_name.to_string(), 
        RankType::Senior, 
        bond_currency,
        10_000.0,
        false, 
        issuedate.clone(), 
        issuedate.clone(),
        None,
        maturity,
        Some(0.03),
        None, 
        None,
        None,
        calendar,
        true,
        DayCountConvention::StreetConvention,
        BusinessDayConvention::Unadjusted, 
        PaymentFrequency::SemiAnnually, 
        0, 
        0,
        bond_name.to_string(), 
        bond_code.to_string(),
    )?;

    let issuedate2 = datetime!(2022-12-10 16:30:00 +09:00);
    let maturity2 = datetime!(2025-12-10 16:30:00 +09:00);
    let issuer_name2 = "Korea Gov";
    let bond_name2 = "국고채권 04250-2512(22-13)";
    let bond_code2 = "KR103501GCC0";
    let sk = Calendar::SouthKorea(SouthKorea::new(SouthKoreaType::Settlement));
    let calendar = JointCalendar::new(vec![sk])?;

    let bond_currency2 = Currency::KRW;
    let issuer_type2 = IssuerType::Government;
    let credit_rating2 = CreditRating::None;

    let bond2 = Bond::new_from_conventions(
        issuer_type2,
        credit_rating2,     
        issuer_name2.to_string(), 
        RankType::Senior, 
        bond_currency2,
        10_000.0, 
        false, 
        issuedate2.clone(), 
        issuedate2.clone(),
        None,
        maturity2,
        Some(0.0425),
        None,
        None, 
        None,
        calendar,
        true,
        DayCountConvention::StreetConvention,
        BusinessDayConvention::Unadjusted, 
        PaymentFrequency::SemiAnnually, 
        0, 
        0,
        bond_name2.to_string(), 
        bond_code2.to_string(),
    )?;

    // option
    let option1 = VanillaOption::new(
        285.0,
        250_000.0,
        datetime!(2021-01-01 00:00:00 +09:00),
        datetime!(2024-09-13 00:00:00 +09:00),
        datetime!(2024-09-13 00:00:00 +09:00),
        datetime!(2024-09-13 00:00:00 +09:00),
        vec![String::from("KOSPI2")],
        Currency::KRW,
        Currency::KRW,
        OptionType::Put,
        OptionExerciseType::European,
        OptionDailySettlementType::NotSettled,
        "KOSPI2 Call Sep21".to_string(),
        "165XXX3".to_string(),
    );

    let inst1 = Instrument::Futures(stock_futures1);
    let inst2 = Instrument::Futures(stock_futures2);
    let inst3: Instrument = Instrument::Bond(bond);
    let inst4: Instrument = Instrument::Bond(bond2);
    let inst5 = Instrument::VanillaOption(option1);

    let inst_vec = vec![inst1, inst2, inst3, inst4, inst5];
    /*
    let inst_vec = vec![
        Rc::new(inst1), 
        Rc::new(inst2), 
        Rc::new(inst3),
        Rc::new(inst4),
        Rc::new(inst5),
        ];
     */
    // make a calculation configuration
    let calculation_configuration = CalculationConfiguration::default()
        .with_delta_calculation(true)
        .with_gamma_calculation(true)
        .with_rho_calculation(true)
        .with_vega_calculation(true)
        .with_vega_structure_calculation(true)
        .with_div_delta_calculation(true)
        .with_rho_structure_calculation(true)
        .with_theta_calculation(true)
        .with_div_structure_calculation(true)
        .with_vega_matrix_calculation(true)
        .with_theta_day(theta_day);
        
    // make a match parameter
    let mut collateral_curve_map = HashMap::new();
    collateral_curve_map.insert(String::from("KOSPI2"), String::from("KSD"));

    let mut borrowing_curve_map = HashMap::new();
    borrowing_curve_map.insert(String::from("KOSPI2"), String::from("KOSPI2"));

    let mut bond_discount_curve_map = HashMap::new();
    bond_discount_curve_map.insert(
        (issuer_name.to_string(), issuer_type, credit_rating, bond_currency), "KRWGOV".to_string()
    );

    let rate_index_curve_map = HashMap::new();

    let mut crs_curve_map = HashMap::new();
    crs_curve_map.insert(Currency::KRW, "KRWCRS".to_string());
    crs_curve_map.insert(Currency::USD, "USDOIS".to_string());

    let mut funding_cost_map = HashMap::new();
    funding_cost_map.insert(Currency::KRW, funding_curve1.clone());

    let match_parameter = MatchParameter::new(
        collateral_curve_map,
        borrowing_curve_map,
        bond_discount_curve_map,
        crs_curve_map,
        rate_index_curve_map,
        funding_cost_map,        
    );

    // make an engine
    let mut engine = Engine::builder(
        0,
        calculation_configuration.clone(),
        evaluation_date.clone(),
        match_parameter.clone(),
    ).with_instruments(inst_vec)?
    .with_parameter_data(
        Arc::new(fx_data_map),
        Arc::new(stock_data_map),
        Arc::new(zero_curve_map),
        Arc::new(dividend_data_map),
        Arc::new(equity_vol_map),
        Arc::new(equity_surface_map),
        Arc::new(HashMap::new()),
        Arc::new(HashMap::new()),
        Arc::new(HashMap::new()),
    )?;
    engine.initialize_pricers().context("failed to initialize pricers")?;
    engine.calculate().context("Failed to calculate")?;

    /* 
    let results = engine.get_calculation_result();
    for (key, value) in results.iter() {
        println!("{}: {:?}\n\n", key, value.borrow());
    }

    let result1 = engine.get_calculation_result().get(&String::from("165XXX1")).unwrap();
    println!("result1 {:?}\n", result1.borrow());

    let result2 = engine.get_calculation_result().get(&String::from("165XXX2")).unwrap();
    println!("result2 {:?}\n", result2.borrow());

    let result3 = engine.get_calculation_result().get(&String::from(bond_code)).unwrap();
    println!("result3 {:?}\n", result3.borrow());

    let result4 = engine.get_calculation_result().get(&String::from(bond_code2)).unwrap();
    println!("result4 {:?}\n", result4.borrow());
    */

    let result5 = engine.get_calculation_result().get(&String::from("165XXX3")).unwrap();
    println!("result5 {:?}\n", result5.borrow());
    
    /*
    println!("\n165XXX1");
    println!("result1 value: {:?}", result1.borrow().get_value());
    println!("result1 delta: {:?}", result1.borrow().get_delta());
    println!("result1 gamma: {:?}", result1.borrow().get_gamma());
    println!("result1 theta: {:?}", result1.borrow().get_theta());
    println!("result1 rho: {:?}", result1.borrow().get_rho());
    println!("result1 rho-structure: {:?}", result1.borrow().get_rho_structure());
    println!("result1 div-delta: {:?}", result1.borrow().get_div_delta());
    println!("result1 div-structure: {:?}", result1.borrow().get_div_structure());

    println!("\n165XXX2");
    println!("result2 value: {:?}", result2.borrow().get_value());
    println!("result2 delta: {:?}", result2.borrow().get_delta());
    println!("result2 gamma: {:?}", result2.borrow().get_gamma());
    println!("result2 theta: {:?}", result2.borrow().get_theta());
    println!("result2 rho: {:?}", result2.borrow().get_rho());
    println!("result2 rho-structure: {:?}", result2.borrow().get_rho_structure());
    println!("result2 div-delta: {:?}", result2.borrow().get_div_delta());
    println!("result2 div-structure: {:?}", result2.borrow().get_div_structure());

    println!("\nKR1234567890");
    println!("result3 value: {:?}", result3.borrow().get_value());
    println!("result3 delta: {:?}", result3.borrow().get_delta());
    println!("result3 gamma: {:?}", result3.borrow().get_gamma());
    println!("result3 theta: {:?}", result3.borrow().get_theta());
    println!("result3 rho: {:?}", result3.borrow().get_rho());
    println!("result3 rho-structure: {:?}", result3.borrow().get_rho_structure());
    println!("result3 div-delta: {:?}", result3.borrow().get_div_delta());
    println!("result3 div-structure: {:?}", result3.borrow().get_div_structure());

    println!("\nKR103501GCC0");
    println!("result4 value: {:?}", result4.borrow().get_value());
    println!("result4 delta: {:?}", result4.borrow().get_delta());
    println!("result4 gamma: {:?}", result4.borrow().get_gamma());
    println!("result4 theta: {:?}", result4.borrow().get_theta());
    println!("result4 rho: {:?}", result4.borrow().get_rho());
    println!("result4 rho-structure: {:?}", result4.bo
    rrow().get_rho_structure());
    println!("result4 div-delta: {:?}", result4.borrow().get_div_delta());
    println!("result4 div-structure: {:?}", result4.borrow().get_div_structure());

    println!("result4 cashflow: {:?}", result4.borrow().get_cashflows());
    //println!("\n\n{:?}", result1);
    // println!("result1:\n{}", serde_json::to_string_pretty(&result1).unwrap());
    // println!("result2:\n{}", serde_json::to_string_pretty(&result2).unwrap());
    */

    let elapsed = start_time.elapsed();
    info!("engine example finished {:?}", elapsed);
    Ok(())
}