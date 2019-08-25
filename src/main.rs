#[macro_use]
extern crate clap;

use clap::{App, Arg, SubCommand};
use std::convert::TryInto;
use std::fmt::{self, Display, Error, Formatter};

#[derive(Debug)]
struct List(Vec<PeriodInfo>);

impl fmt::Display for List {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        let vec = &self.0;

        for v in vec.iter() {
            writeln!(f, "{}", v)?;
        }
        Ok(())
    }
}

#[derive(Debug)]
struct LoanInfo {
    principal: f64,
    rate: f64,
    period: i32,
}

impl LoanInfo {
    fn payment(&self) -> f64 {
        let rate = self.rate / 12.;
        let num = rate * (1. + rate).powi(self.period);
        let denom = ((1. + rate).powi(self.period)) - 1.;
        self.principal * (num / denom)
    }

    fn monthly_payment(&self) -> f64 {
        self.payment() / 12 as f64
    }

    pub fn new(principal: f64, rate: f64, period: i32) -> LoanInfo {
        LoanInfo {
            principal,
            rate,
            period,
        }
    }
}

#[derive(Debug)]
struct PeriodInfo {
    month: u32,
    upb: f64,
    interest: f64,
    principal: f64,
    ending_upb: f64,
}

impl fmt::Display for LoanInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "principal: {}, rate: {}, period: {}, payment: {}, monthly payment: {}",
            self.principal,
            self.rate,
            self.period,
            self.payment(),
            self.monthly_payment()
        )
    }
}

impl fmt::Display for PeriodInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "month: {}, starting UPB: {}, interest payment: {}, principal payment: {}, ending UPB: {}",
               self.month, self.upb, self.interest, self.principal, self.ending_upb)
    }
}

fn amort_period(loan: &mut LoanInfo, n: i32, payment: f64) -> PeriodInfo {
    println!("{}", payment);
    let interest = loan.principal * (loan.rate / 12.);
    let upb = *&loan.principal;
    let principal = payment - interest;
    loan.principal -= principal;
    PeriodInfo {
        month: n.try_into().expect("Unable to convert period to months"),
        upb,
        interest,
        principal,
        ending_upb: loan.principal,
    }
}

fn amort(loan: &mut LoanInfo) -> List {
    let payment = loan.payment();
    let mut ret = Vec::new();
    for x in 1..=loan.period {
        ret.push(amort_period(loan, x, payment));
    }
    List(ret)
}

fn main() {
    let matches = App::new("Echo")
        .version("0.0.1")
        .author("KP")
        .arg(
            Arg::with_name("principal")
                .short("p")
                .long("principal")
                .help("principal amount")
                .required(true)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("rate")
                .short("r")
                .long("rate")
                .help("interest rate")
                .required(true)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("period")
                .short("n")
                .long("period")
                .help("length of loan")
                .required(true)
                .takes_value(true),
        )
        .get_matches();
    let loan = &mut LoanInfo {
        principal: value_t!(matches, "principal", f64).unwrap(),
        rate: value_t!(matches, "rate", f64).unwrap(),
        period: value_t!(matches, "period", i32).unwrap(),
    };
    println!("{}", loan);

    let amort = amort(loan);
    println!("{}", amort);
}
