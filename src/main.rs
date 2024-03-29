#[cfg(test)]
extern crate assert_cmd;
#[macro_use]
extern crate clap;
#[cfg(test)]
#[macro_use]
extern crate float_cmp;
#[cfg(test)]
extern crate tempfile;

use std::convert::TryInto;
use std::error::Error;
use std::fmt::{self, Formatter};
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
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
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            OutputType::File(ref c) => write!(f, "File: {}", c),
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
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
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
    let period = value_t!(matches, "periods", i32).unwrap();
    let mut output: Option<&str> = None;
    if let Some(o) = matches.value_of("output") {
        println!("Using output file: {:?}", o);
        output = Some(o);
    }
    let output_type: OutputType = match output {
        Some(ref e) => OutputType::File(e.to_string()),
        None => OutputType::Stdout,
    };
    let out_path = match output_type {
        OutputType::File(ref p) => Some(Path::new(p)),
        OutputType::Stdout => None,
    };

    println!("output type: {}", output_type);

    let loan = &mut LoanInfo::new(principal, rate, period);
    println!("{}", loan);

    let amort = amort(loan);
    if let Some(p) = out_path {
        let display = p.display();
        let mut file = match File::create(p) {
            Err(why) => panic!("Couldn't create {}: {}", display, why.description()),
            Ok(file) => file,
        };

        match file.write_all(amort.to_string().as_bytes()) {
            Err(why) => panic!("Couldn't write to {}: {}", display, why.description()),
            Ok(_) => println!("Successfully wrote to {}", display),
        }
    } else {
        println!("{}", amort);
    }
}

#[cfg(test)]
mod test {
    use std::fs::File;
    use std::io::{self, BufRead, BufReader, Read, Write};
    use std::process::Command;

    use assert_cmd::prelude::*;
    use itertools::enumerate;
    use tempfile::NamedTempFile;

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

    #[test]
    fn test_cli_file_output() {
        let mut f = NamedTempFile::new().unwrap();
        let path = f.into_temp_path();
        let path_str = path.as_os_str();
        let mut cmd = Command::cargo_bin("amort").unwrap();
        cmd.arg("-p")
            .arg("100")
            .arg("-r")
            .arg(".0123")
            .arg("-n")
            .arg("360")
            .arg("-o")
            .arg(path_str);
        let output = cmd.output().unwrap();
        let stdout = output.stdout;
        println!("stdout: {:?}", stdout);
        let assert = cmd.assert();
        assert.success();
        let input = File::open(path).unwrap();
        let buf = BufReader::new(input);

        let mut i = 0;
        for _line in buf.lines() {
            i += 1;
        }
        assert_eq!(i, 360);
    }
}
