#[macro_use]
extern crate clap;
#[cfg(test)]
#[macro_use]
extern crate float_cmp;

use std::convert::TryInto;
use std::fmt::{self, Error, Formatter};
use std::str::FromStr;

use clap::{App, Arg};

#[derive(Debug)]
struct List(Vec<PeriodInfo>);

#[derive(PartialEq, Debug)]
pub enum OutputType {
    File(String),
    Stdout,
}

impl fmt::Display for OutputType {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        match self {
            OutputType::File(c) => write!(f, "File: {}", c),
            OutputType::Stdout => write!(f, "stdout"),
        }
    }
}

impl List {
    fn last(&mut self) -> Option<&PeriodInfo> {
        let vec = &self.0;
        vec.last()
    }
}

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
    payment: f64,
}

impl LoanInfo {
    pub fn new(principal: f64, rate: f64, period: i32) -> LoanInfo {
        let monthly_rate: f64 = {
            if rate > 1. {
                (rate / 100.) / 12.
            } else {
                rate / 12.
            }
        };

        let payment = payment(monthly_rate, period, principal);

        LoanInfo {
            principal,
            rate: monthly_rate,
            period,
            payment,
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
            "principal: {}, rate: {}, period: {}, payment: {}",
            self.principal, self.rate, self.period, self.payment,
        )
    }
}

impl fmt::Display for PeriodInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "month: {}, starting UPB: {}, interest payment: {}, principal payment: {}, ending UPB: {}",
               self.month, self.upb, self.interest, self.principal, self.ending_upb)
    }
}

fn payment(rate: f64, period: i32, principal: f64) -> f64 {
    let num = rate * (1. + rate).powi(period);
    let denom = ((1. + rate).powi(period)) - 1.;
    principal * (num / denom)
}

fn amort_period(loan: &mut LoanInfo, n: i32, payment: f64) -> PeriodInfo {
    let interest = loan.principal * loan.rate;
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
    let payment = loan.payment;
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
                .help("principal amount of the loan")
                .required(true)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("rate")
                .short("r")
                .long("interest-rate")
                .help("the annual interest rate")
                .required(true)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("periods")
                .short("n")
                .long("periods")
                .help("length of loan in terms of months")
                .required(true)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("output")
                .short("o")
                .long("output")
                .takes_value(true),
        )
        .get_matches();

    let principal = value_t!(matches, "principal", f64).unwrap();
    let rate = value_t!(matches, "rate", f64).unwrap();
    let period = value_t!(matches, "period", i32).unwrap();
    let output: Option<&str> = Option::from(matches.value_of("output").unwrap());
    let output_type: OutputType = match output {
        Some(ref e) => OutputType::File(e.to_string()),
        None => OutputType::Stdout,
    };

    println!("output type: {}", output_type);

    let loan = &mut LoanInfo::new(principal, rate, period);
    println!("{}", loan);

    let amort = amort(loan);
    println!("{}", amort);
}

#[cfg(test)]
mod test {
    use itertools::enumerate;

    use crate::{amort, amort_period, LoanInfo};

    #[test]
    fn test_amort_period() {
        let loan = &mut LoanInfo::new(100_000., 0.05, 360);
        println!("{}", loan);
        let payment = loan.payment;
        let period_info = amort_period(loan, 1, payment);
        assert!(approx_eq!(
            f64,
            period_info.interest,
            416.67,
            epsilon = 0.01
        ));
        assert!(approx_eq!(
            f64,
            period_info.principal,
            120.15,
            epsilon = 0.01
        ));
        assert!(approx_eq!(
            f64,
            period_info.principal + period_info.interest,
            payment,
            epsilon = 0.01
        ));
        assert!(approx_eq!(
            f64,
            loan.principal,
            period_info.ending_upb,
            epsilon = 0.01
        ));
        println!("{}", period_info)
    }

    #[test]
    fn test_amort() {
        let periods = 360;
        let loan = &mut LoanInfo::new(100_000., 0.05, periods);
        let payment = loan.payment;
        let mut period_infos = amort(loan);
        let vec = &period_infos.0;
        for (i, p) in enumerate(vec) {
            assert!(approx_eq!(
                f64,
                p.principal + p.interest,
                payment,
                epsilon = 0.01
            ))
        }
        match period_infos.last() {
            Some(ref p) => assert!(approx_eq!(f64, p.ending_upb, 0f64, epsilon = 0.01)),
            None => panic!("No 'last' value found"),
        }
    }
}
