#![allow(dead_code)]
#![allow(unused_variables)]

use rppal::gpio::Gpio;
use rppal::gpio::Trigger::FallingEdge;

use rppal::i2c::I2c;
//use rppal::pwm::{Channel, Pwm};
use rppal::spi::{Bus, Mode, SlaveSelect, Spi};
//use rppal::uart::{Parity, Uart};
use std::thread;
use std::time::Duration;
use std::error::Error;
use std::process::Command;
use std::str;

use std::str::FromStr;
use std::collections::VecDeque;
use std::cmp::{min, max};


extern crate daemonize;
use std::fs::File;
use daemonize::Daemonize;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    daemonize: bool,
}


pub struct Hw {
    pub gpio: Gpio,
    pub i2c: I2c,
    //pub pwm: Pwm,
    pub spi: Spi,
    //pub uart: Uart
    pub brightness: Brightness,

}

impl Hw {
    pub fn new() -> Hw {
        Hw {
            gpio: Gpio::new().unwrap(),
            i2c: I2c::new().unwrap(),
            // pwm: Pwm::new(Channel::Pwm0).unwrap(),
            spi: Spi::new(Bus::Spi0, SlaveSelect::Ss0, 16_000_000, Mode::Mode0).unwrap(),
            //uart: Uart::new(115_200, Parity::None, 8, 1).unwrap(),
            brightness: Brightness::new(),
            }
        }
    }

    
const DP1: &str = "0x0f";
const DP2: &str = "0x10";
// const JOY_UP: u8= 6;
// const JOY_DOWN: u8= 19;
const JOY_LEFT: u8= 5;
const JOY_RIGHT: u8= 26;
// const JOY_PRESS: u8= 13;
const KEY1: u8= 21;
// const KEY2: u8= 20;
const KEY3: u8= 16;
// const MOSI: u8 = 10;
// const SCLK: u8 = 11;
// const CS: u8 = 8;
// const DC: u8 = 25;
// const RST: u8 = 27;
const BL: u8 = 24;


pub struct Brightness {
    pub brightness: u8,
}

impl Brightness {
    pub fn new() -> Brightness {
        Brightness {
            brightness: 50
        }
    }

    pub fn get_brightness(&mut self) -> Result<u8, Box<dyn Error>> {
            let mut cmd = Command::new("ddcutil");
            let cmd = cmd.args(["getvcp", "10"]);
        
            let output = cmd.output()
                                    .expect("failed to execute process");
            let s = match str::from_utf8(&output.stdout) {
                Ok(v) => v,
                Err(e) => panic!("Invalid utf-8: {}", e)
            };

            let mut s = s.split('=').collect::<VecDeque<_>>();
            _= s.pop_front();
            let s = s.pop_front().unwrap().trim();
            let s = s.split(',').collect::<VecDeque<_>>().pop_front().unwrap();
            println!("found: {}", s);
            let val = u8::from_str(s).unwrap();
                      
            self.brightness = val;
            Ok(val)    
    }

    pub fn compute_brightness(&self, sign: char, val: u8) -> u8 {
        match sign {
            '-' => max(self.brightness + val, 0),
            '+' => min(self.brightness + val, 100),
            _ => min(max(val, 0), 100),
        }
    }

    pub fn set_brightness(&mut self, sign: char, val: u8) {
        let new_val = self.compute_brightness(sign, val);
        let mut cmd = Command::new("ddcutil");
        let cmd = cmd.args(["setvcp", "10", &val.to_string()]);
    
        let _output = cmd.output()
                                .expect("failed to execute process");
    

    }


}

fn setdp(dp: &str)  {
    let mut cmd = Command::new("ddcutil");
    let cmd = cmd.args(["setvcp", "60", &dp]);

    let _output = cmd.output()
                            .expect("failed to execute process");

    
}

fn getdp() -> Result<String, Box<dyn Error>> {
    let mut cmd = Command::new("ddcutil");
    let cmd = cmd.args(["getvcp", "60"]);

    let output = cmd.output()
                            .expect("failed to execute process");
    let s = match str::from_utf8(&output.stdout) {
        Ok(v) => v,
        Err(e) => panic!("Invalid utf-8: {}", e)
    };
    Ok(s.to_string())
}

fn daemonize() {
    let stdout = File::create("log").unwrap();
    let stderr = File::create("log.err").unwrap();

    let daemonize = Daemonize::new()
        .umask(0o077)    // Set umask, `0o027` by default.
        .stdout(stdout)  // Redirect stdout to `/tmp/daemon.out`.
        .stderr(stderr)  // Redirect stderr to `/tmp/daemon.err`.
        .privileged_action(|| "Executed before drop privileges");

    match daemonize.start() {
        Ok(_) => println!("Success, daemonized"),
        Err(e) => eprintln!("Error, {}", e),
    }

}


fn main() -> Result<(), Box<dyn Error>> {

    let args = Args::parse();
    if args.daemonize == true {
        daemonize();
    }

    let mut hwconfig = Hw::new();
    
    let mut key1 = hwconfig.gpio.get(KEY1)?.into_input_pullup();
    let mut key3 = hwconfig.gpio.get(KEY3)?.into_input_pullup();
    // let mut joyleft = hwconfig.gpio.get(JOY_LEFT)?.into_input_pullup();
    // let mut joyright = hwconfig.gpio.get(JOY_RIGHT)?.into_input_pullup();

    // there might be a bounce that sometimes randomly switch inputs?
    // wait here before setting the interrupts
    thread::sleep(Duration::from_millis(100)); 

    let _ = key1.set_async_interrupt(FallingEdge, |_| { setdp(DP1); println!("{}", getdp().unwrap()); })
                .expect("Could not configure Key1");
    let _ = key3.set_async_interrupt(FallingEdge, |_| { setdp(DP2); println!("{}", getdp().unwrap()); })
                .expect("Could not configure Key3");
    // let _ = joyleft.set_async_interrupt(RisingEdge, |_| { hwconfig.brightness.set_brightness('-', 50) })
    //             .expect("Could not configure JOY LEFT");

    // let _ = joyright.set_async_interrupt(RisingEdge, |_| { hwconfig.brightness.set_brightness('+', 50) })
    //             .expect("Could not configure JOY RIGHT");


     // turn off the LCD
    let mut bl_p = hwconfig.gpio.get(BL)?.into_output();
    bl_p.set_low();

    println!("{:?}",hwconfig.brightness.get_brightness());
    //   
    println!("{}", getdp().unwrap());
    println!("Listening for key commands");

    // never exit
    loop {thread::sleep(Duration::from_millis(30000));};
}

