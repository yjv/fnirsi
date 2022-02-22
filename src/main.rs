use std::convert::{TryFrom, TryInto};
use std::fmt::{Debug, Display, Formatter};
use binread::{BinRead, BinReaderExt, io::SeekFrom};
use std::fs::File as FsFile;
use std::io::stdout;
use std::str::FromStr;
use lazy_static::lazy_static;
use serde::{Serialize, Serializer};
use clap::{Parser, ArgEnum};
use num_enum::{TryFromPrimitive, TryFromPrimitiveError};
use thiserror::Error;

const DIVISION_POINTS: f32 = 50.0;
const VOLTAGE_MEASUREMENT_DIVISOR: f32 = 1024f32;

lazy_static! {
    /// This is an example for using doc comment attributes
    static ref TIME_SCALES: Vec<Scale<Second>> = vec![
        Scale { value: 50.0, scale: 0, unit: Second },
        Scale { value: 20.0, scale: 0, unit: Second },
        Scale { value: 10.0, scale: 0, unit: Second },
        Scale { value: 5.0, scale: 0, unit: Second },
        Scale { value: 2.0, scale: 0, unit: Second },
        Scale { value: 1.0, scale: 0, unit: Second },
        Scale { value: 500.0, scale: -3, unit: Second },
        Scale { value: 200.0, scale: -3, unit: Second },
        Scale { value: 100.0, scale: -3, unit: Second },
        Scale { value: 50.0, scale: -3, unit: Second },
        Scale { value: 20.0, scale: -3, unit: Second },
        Scale { value: 10.0, scale: -3, unit: Second },
        Scale { value: 5.0, scale: -3, unit: Second },
        Scale { value: 2.0, scale: -3, unit: Second },
        Scale { value: 1.0, scale: -3, unit: Second },
        Scale { value: 500.0, scale: -6, unit: Second },
        Scale { value: 200.0, scale: -6, unit: Second },
        Scale { value: 100.0, scale: -6, unit: Second },
        Scale { value: 50.0, scale: -6, unit: Second },
        Scale { value: 20.0, scale: -6, unit: Second },
        Scale { value: 10.0, scale: -6, unit: Second },
        Scale { value: 5.0, scale: -6, unit: Second },
        Scale { value: 2.0, scale: -6, unit: Second },
        Scale { value: 1.0, scale: -6, unit: Second },
        Scale { value: 500.0, scale: -9, unit: Second },
        Scale { value: 200.0, scale: -9, unit: Second },
        Scale { value: 100.0, scale: -9, unit: Second },
        Scale { value: 50.0, scale: -9, unit: Second },
        Scale { value: 20.0, scale: -9, unit: Second },
        Scale { value: 10.0, scale: -9, unit: Second },
        Scale { value: 5.0, scale: -9, unit: Second },
        Scale { value: 2.0, scale: -9, unit: Second },
        Scale { value: 1.0, scale: -9, unit: Second },
    ];
}

lazy_static! {
    /// This is an example for using doc comment attributes
    static ref PROBE_SCALES: Vec<Scale<Volt>> = vec![
        Scale { value: 5.0, scale: 0, unit: Volt },
        Scale { value: 2.5, scale: 0, unit: Volt },
        Scale { value: 1.0, scale: 0, unit: Volt },
        Scale { value: 500.0, scale: -3, unit: Volt },
        Scale { value: 200.0, scale: -3, unit: Volt },
        Scale { value: 100.0, scale: -3, unit: Volt },
        Scale { value: 50.0, scale: -3, unit: Volt },
    ];
}

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    output: Output,
    file: String
}

#[derive(Debug, ArgEnum, Clone)]
enum Output {
    #[clap(name = "raw")]
    Raw,
    #[clap(name = "parsed")]
    Parsed
}

impl FromStr for Output {
    type Err = OutputParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "raw" => Output::Raw,
            "parsed" => Output::Parsed,
            other => return Err(OutputParseError(other.to_string()))
        })
    }
}

#[derive(Debug, Error)]
#[error("The output type {0} is not supported")]
struct OutputParseError(String);

fn main() {
    let args = Args::parse();
    let file: File = FsFile::open(args.file).unwrap().read_le().unwrap();

    match args.output {
        Output::Raw => serde_json::to_writer(stdout(), &file),
        Output::Parsed => {
            let time_scale = file.header.time_scale.try_into().unwrap();
            let chanel1_scale = file.header.channel1_scale.try_into().unwrap();
            let channel2_scale = file.header.channel2_scale.try_into().unwrap();
            let channel1_points = generate_points(&file.channel11, &chanel1_scale, &time_scale, file.header.channel1_offset);
            let channel2_points = generate_points(&file.channel11, &channel2_scale, &time_scale, file.header.channel2_offset);

            let data = Data {
                trigger: Trigger {
                    trigger_type: file.header.trigger_type.try_into().unwrap(),
                    edge: file.header.trigger_edge.try_into().unwrap(),
                    channel: file.header.trigger_channel.try_into().unwrap(),
                    trigger_50: file.header.trigger_50.try_into().unwrap()
                },
                time_scale,
                channel1: Channel {
                    scale: chanel1_scale,
                    coupling: file.header.channel1_coupling.try_into().unwrap(),
                    attenuation: file.header.channel1_probe.try_into().unwrap(),
                    measurements: ProcessedMeasurements {
                        vmax: process_voltage_measurement(file.header.channel1_measurements.vmax),
                        vmin: process_voltage_measurement(file.header.channel1_measurements.vmin),
                        vavg: process_voltage_measurement(file.header.channel1_measurements.vavg),
                        vrms: process_voltage_measurement(file.header.channel1_measurements.vrms),
                        vpp: process_voltage_measurement(file.header.channel1_measurements.vpp),
                        vp: process_voltage_measurement(file.header.channel1_measurements.vp),
                        frequency: parse_frequency(file.header.channel1_measurements.frequency_high, file.header.channel1_measurements.frequency_low),
                        cycle_ns: file.header.channel1_measurements.cycle_ns,
                        time_plus_ns: file.header.channel1_measurements.time_plus_ns,
                        time_minus_ns: file.header.channel1_measurements.time_minus_ns,
                        duty_plus_percentage: file.header.channel1_measurements.duty_plus_percentage,
                        duty_minus_percentage: file.header.channel1_measurements.duty_minus_percentage
                    },
                    points: channel1_points
                },
                channel2: Channel {
                    scale: channel2_scale,
                    coupling: file.header.channel2_coupling.try_into().unwrap(),
                    attenuation: file.header.channel2_probe.try_into().unwrap(),
                    measurements: ProcessedMeasurements {
                        vmax: process_voltage_measurement(file.header.channel2_measurements.vmax),
                        vmin: process_voltage_measurement(file.header.channel2_measurements.vmin),
                        vavg: process_voltage_measurement(file.header.channel2_measurements.vavg),
                        vrms: process_voltage_measurement(file.header.channel2_measurements.vrms),
                        vpp: process_voltage_measurement(file.header.channel2_measurements.vpp),
                        vp: process_voltage_measurement(file.header.channel2_measurements.vp),
                        frequency: parse_frequency(file.header.channel2_measurements.frequency_high, file.header.channel2_measurements.frequency_low),
                        cycle_ns: file.header.channel2_measurements.cycle_ns,
                        time_plus_ns: file.header.channel2_measurements.time_plus_ns,
                        time_minus_ns: file.header.channel2_measurements.time_minus_ns,
                        duty_plus_percentage: file.header.channel2_measurements.duty_plus_percentage,
                        duty_minus_percentage: file.header.channel2_measurements.duty_minus_percentage
                    },
                    points: channel2_points
                }
            };

            serde_json::to_writer(stdout(), &data)
        }
    }.unwrap();
}

fn parse_frequency(high: u16, low: u16) -> u32 {
    ((high as u32) << 16) + low as u32
}

#[derive(Debug, Serialize)]
struct Data {
    trigger: Trigger,
    time_scale: Scale<Second>,
    channel1: Channel,
    channel2: Channel,
}

#[derive(Debug, Serialize)]
struct Channel {
    scale: Scale<Volt>,
    coupling: Coupling,
    attenuation: Attenuation,
    measurements: ProcessedMeasurements,
    points: Vec<Point>
}

#[derive(Debug, Serialize)]
struct Trigger {
    trigger_type: TriggerType,
    edge: TriggerEdge,
    channel: TriggerChannel,
    trigger_50: Trigger50
}

fn generate_points(values: &Vec<u16>, voltage_scale: &Scale<Volt>, time_scale: &Scale<Second>, offset: u16) -> Vec<Point> {
    values.iter().enumerate().map(| (index, voltage)| Point {
        time: (index as f32) * time_scale.get_scale()/ DIVISION_POINTS,
        voltage: (*voltage as f32 - offset as f32) * voltage_scale.get_scale()/DIVISION_POINTS
    }).collect()
}

fn process_voltage_measurement(measurement: u16) -> f32 {
    (measurement as f32)/VOLTAGE_MEASUREMENT_DIVISOR
}

#[derive(Debug, Serialize)]
struct Point {
    time: f32,
    voltage: f32
}

#[derive(BinRead, Debug, Serialize)]
#[br(little)]
pub struct File {
    header: Header,
    #[br(count = 1500, seek_before = SeekFrom::Start(1000))]
    channel11: Vec<u16>,
    #[br(count = 1500)]
    channel21: Vec<u16>,
    #[br(count = 750)]
    channel12: Vec<u16>,
    #[br(count = 750)]
    channel22: Vec<u16>
}

#[derive(BinRead, Debug, Serialize)]
pub struct Header {
    #[br(pad_before = 4)]
    channel1_scale: u16,
    #[br(pad_before = 2)]
    channel1_coupling: u16,
    channel1_probe: u16,
    #[br(pad_before = 2)]
    channel2_scale: u16,
    #[br(pad_before = 2)]
    channel2_coupling: u16,
    channel2_probe: u16,
    time_scale: u16,
    scroll_speed: u16,
    trigger_type: u16,
    trigger_edge: u16,
    trigger_channel: u16,
    #[br(pad_before = 52)]
    channel1_offset: u16,
    channel2_offset: u16,
    #[br(pad_before = 32)]
    screen_brightness: u16,
    grid_brightness: u16,
    trigger_50: u16,
    #[br(seek_before = SeekFrom::Start(208))]
    channel1_measurements: Measurements,
    #[br(seek_before = SeekFrom::Start(256))]
    channel2_measurements: Measurements
}

#[derive(BinRead, Debug, Serialize)]
#[br(little)]
pub struct Measurements {
    #[br(pad_before = 2)]
    vmax: u16,
    #[br(pad_before = 2)]
    vmin: u16,
    #[br(pad_before = 2)]
    vavg: u16,
    #[br(pad_before = 2)]
    vrms: u16,
    #[br(pad_before = 2)]
    vpp: u16,
    #[br(pad_before = 2)]
    vp: u16,
    frequency_high: u16,
    frequency_low: u16,
    #[br(pad_before = 2)]
    cycle_ns: u16,
    #[br(pad_before = 2)]
    time_plus_ns: u16,
    #[br(pad_before = 2)]
    time_minus_ns: u16,
    #[br(pad_before = 2)]
    duty_plus_percentage: u16,
    #[br(pad_before = 2)]
    duty_minus_percentage: u16
}

#[derive(Debug, Serialize)]
pub struct ProcessedMeasurements {
    vmax: f32,
    vmin: f32,
    vavg: f32,
    vrms: f32,
    vpp: f32,
    vp: f32,
    frequency: u32,
    cycle_ns: u16,
    time_plus_ns: u16,
    time_minus_ns: u16,
    duty_plus_percentage: u16,
    duty_minus_percentage: u16
}

trait Unit: Display + Clone + Copy {}

#[derive(Clone, Copy)]
struct Volt;

impl Unit for Volt {}

impl Serialize for Volt {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        str::serialize("Volt", serializer)
    }
}

impl Display for Volt {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "V")
    }
}

#[derive(Clone, Copy)]
struct Second;

impl Unit for Second {}

impl Serialize for Second {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        str::serialize("Second", serializer)
    }
}

impl Display for Second {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "s")
    }
}

#[derive(Clone, Serialize, Copy)]
struct Scale<T: Unit> {
    value: f32,
    scale: i32,
    unit: T
}

impl <T: Unit> Scale<T> {
    fn get_scale(&self) -> f32 {
        self.value * 10_f32.powi(self.scale)
    }
}

impl <T: Unit> Display for Scale<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}{}", self.value, match self.scale {
            0 => "",
            -3 => "m",
            -6 => "u",
            -9 => "n",
            other => unreachable!("Unexpected scale {}", other)
        }, self.unit)
    }
}

impl <T: Unit> Debug for Scale<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Scale {{ value {}, scale: {}, string: {} }}", self.value, self.scale, self)
    }
}

impl TryFromPrimitive for Scale<Volt> {
    type Primitive = u16;
    const NAME: &'static str = "Scale<Volt>";

    fn try_from_primitive(number: Self::Primitive) -> Result<Self, TryFromPrimitiveError<Self>> {
        PROBE_SCALES.get(number as usize).map(Clone::clone).ok_or_else(|| TryFromPrimitiveError { number })
    }
}

impl TryFrom<u16> for Scale<Volt> {
    type Error = TryFromPrimitiveError<Self>;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        TryFromPrimitive::try_from_primitive(value)
    }
}

impl TryFromPrimitive for Scale<Second> {
    type Primitive = u16;
    const NAME: &'static str = "Scale<Second>";

    fn try_from_primitive(number: Self::Primitive) -> Result<Self, TryFromPrimitiveError<Self>> {
        TIME_SCALES.get(number as usize).map(Clone::clone).ok_or_else(|| TryFromPrimitiveError { number })
    }
}

impl TryFrom<u16> for Scale<Second> {
    type Error = TryFromPrimitiveError<Self>;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        TryFromPrimitive::try_from_primitive(value)
    }
}

#[derive(Debug, Eq, PartialEq, TryFromPrimitive, Serialize)]
#[repr(u16)]
enum Coupling {
    DC = 0, AC
}

#[derive(Debug, Eq, PartialEq, TryFromPrimitive, Serialize)]
#[repr(u16)]
enum Attenuation {
    OneX = 0,
    TenX,
    OneHundredX
}

#[derive(Debug, Eq, PartialEq, TryFromPrimitive, Serialize)]
#[repr(u16)]
enum ScrollSpeed {
    Fast = 0, Slow
}

#[derive(Debug, Eq, PartialEq, TryFromPrimitive, Serialize)]
#[repr(u16)]
enum TriggerType {
    Auto = 0, Single, Normal
}

#[derive(Debug, Eq, PartialEq, TryFromPrimitive, Serialize)]
#[repr(u16)]
enum TriggerEdge {
    Rising = 0, Falling
}

#[derive(Debug, Eq, PartialEq, TryFromPrimitive, Serialize)]
#[repr(u16)]
enum TriggerChannel {
    Channel1 = 0, Channel2
}

#[derive(Debug, Eq, PartialEq, TryFromPrimitive, Serialize)]
#[repr(u16)]
enum Trigger50 {
    On = 0, Off
}
