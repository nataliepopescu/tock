// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

//! Board file for Imix development platform.
//!
//! - <https://github.com/tock/tock/tree/master/boards/imix>
//! - <https://github.com/tock/imix>

#![no_std]
#![no_main]
#![deny(missing_docs)]

mod imix_components;

pub mod kernel;
#[macro_use]
pub mod capsules_core;
pub mod capsules_extra;
pub mod capsules_system;
pub mod sam4l;
pub mod components;
pub mod cortexm4;
pub mod cortexm;
pub mod cortexv7m;
pub mod segger;

use crate::capsules_core::alarm::AlarmDriver;
use crate::capsules_core::console_ordered::ConsoleOrdered;
use crate::capsules_core::virtualizers::virtual_aes_ccm::MuxAES128CCM;
use crate::capsules_core::virtualizers::virtual_alarm::VirtualMuxAlarm;
use crate::capsules_core::virtualizers::virtual_i2c::MuxI2C;
use crate::capsules_core::virtualizers::virtual_spi::VirtualSpiMasterDevice;
use crate::capsules_extra::net::ieee802154::MacAddress;
use crate::capsules_extra::net::ipv6::ip_utils::IPAddr;
use crate::kernel::capabilities;
use crate::kernel::component::Component;
use crate::kernel::debug::PanicResources;
use crate::kernel::deferred_call::DeferredCallClient;
use crate::kernel::hil::i2c::I2CMaster;
use crate::kernel::hil::radio;
#[allow(unused_imports)]
use crate::kernel::hil::radio::{RadioConfig, RadioData};
use crate::kernel::hil::symmetric_encryption::AES;
use crate::kernel::platform::chip::Chip;
use crate::kernel::platform::{KernelResources, SyscallDriverLookup};
use crate::kernel::utilities::single_thread_value::SingleThreadValue;

//use crate::kernel::hil::time::Alarm;
use crate::kernel::hil::Controller;
use crate::kernel::hil::led::LedHigh;
#[allow(unused_imports)]
use crate::kernel::{create_capability, debug, debug_gpio, static_buf, static_init};
use crate::sam4l::chip::Sam4lDefaultPeripherals;

use crate::components::alarm::{AlarmDriverComponent, AlarmMuxComponent};
use crate::components::console::{ConsoleOrderedComponent, UartMuxComponent};
use crate::components::crc::CrcComponent;
use crate::components::debug_writer::DebugWriterComponent;
use crate::components::gpio::GpioComponent;
use crate::components::isl29035::AmbientLightComponent;
use crate::components::isl29035::Isl29035Component;
use crate::components::led::LedsComponent;
use crate::components::nrf51822::Nrf51822Component;
use crate::components::process_console::ProcessConsoleComponent;
use crate::components::rng::RngComponent;
use crate::components::si7021::SI7021Component;
use crate::components::spi::{SpiComponent, SpiSyscallComponent};

/// Support routines for debugging I/O.
///
/// Note: Use of this module will trample any other USART3 configuration.
pub mod io;

// Unit Tests for drivers.
#[allow(dead_code)]
mod test;

// Helper functions for enabling/disabling power on Imix submodules
mod power;

#[allow(dead_code)]
mod alarm_test;

#[allow(dead_code)]
mod multi_timer_test;

// State for loading apps.

const NUM_PROCS: usize = 4;

// Constants related to the configuration of the 15.4 network stack
// TODO: Notably, the radio MAC addresses can be configured from userland at the moment
// We probably want to change this from a security perspective (multiple apps being
// able to change the MAC address seems problematic), but it is very convenient for
// development to be able to just flash two corresponding apps onto two devices and
// have those devices talk to each other without having to modify the kernel flashed
// onto each device. This makes MAC address configuration a good target for capabilities -
// only allow one app per board to have control of MAC address configuration?
const RADIO_CHANNEL: radio::RadioChannel = radio::RadioChannel::Channel26;
const DST_MAC_ADDR: MacAddress = MacAddress::Short(49138);
const DEFAULT_CTX_PREFIX_LEN: u8 = 8; //Length of context for 6LoWPAN compression
const DEFAULT_CTX_PREFIX: [u8; 16] = [0x0_u8; 16]; //Context for 6LoWPAN Compression
const PAN_ID: u16 = 0xABCD;

// how should the kernel respond when a process faults
const FAULT_RESPONSE: crate::capsules_system::process_policies::StopFaultPolicy =
    crate::capsules_system::process_policies::StopFaultPolicy {};

type ChipHw = crate::sam4l::chip::Sam4l<Sam4lDefaultPeripherals>;
type ProcessPrinterInUse = crate::capsules_system::process_printer::ProcessPrinterText;

// TODO: remove once panic UART doesn't require an existing instance any more
static mut CHIP: Option<&'static crate::sam4l::chip::Sam4l<Sam4lDefaultPeripherals>> = None;

/// Resources for when a board panics used by io.rs.
static PANIC_RESOURCES: SingleThreadValue<PanicResources<ChipHw, ProcessPrinterInUse>> =
    SingleThreadValue::new();

crate::kernel::stack_size! {0x2000}

type SI7021Sensor = crate::components::si7021::SI7021ComponentType<
    crate::capsules_core::virtualizers::virtual_alarm::VirtualMuxAlarm<'static, crate::sam4l::ast::Ast<'static>>,
    crate::capsules_core::virtualizers::virtual_i2c::I2CDevice<'static, crate::sam4l::i2c::I2CHw<'static>>,
>;
type TemperatureDriver = crate::components::temperature::TemperatureComponentType<SI7021Sensor>;
type HumidityDriver = crate::components::humidity::HumidityComponentType<SI7021Sensor>;
type RngDriver = crate::components::rng::RngComponentType<crate::sam4l::trng::Trng<'static>>;

type Rf233 = crate::capsules_extra::rf233::RF233<
    'static,
    VirtualSpiMasterDevice<'static, crate::sam4l::spi::SpiHw<'static>>,
>;
type Ieee802154MacDevice =
    crate::components::ieee802154::Ieee802154ComponentMacDeviceType<Rf233, crate::sam4l::aes::Aes<'static>>;

type SchedulerInUse = crate::components::sched::round_robin::RoundRobinComponentType;

crate::kernel::define_capability_type!(ProcessConsoleCap:
    crate::kernel::capabilities::ProcessManagementCapability,
    crate::kernel::capabilities::ProcessStartCapability
);

struct Imix {
    pconsole: &'static crate::capsules_core::process_console::ProcessConsole<
        'static,
        { crate::capsules_core::process_console::DEFAULT_COMMAND_HISTORY_LEN },
        crate::capsules_core::virtualizers::virtual_alarm::VirtualMuxAlarm<
            'static,
            crate::sam4l::ast::Ast<'static>,
        >,
        ProcessConsoleCap,
    >,
    console: &'static crate::capsules_core::console_ordered::ConsoleOrdered<
        'static,
        VirtualMuxAlarm<'static, crate::sam4l::ast::Ast<'static>>,
    >,
    gpio: &'static crate::capsules_core::gpio::GPIO<'static, crate::sam4l::gpio::GPIOPin<'static>>,
    alarm: &'static AlarmDriver<'static, VirtualMuxAlarm<'static, crate::sam4l::ast::Ast<'static>>>,
    temp: &'static TemperatureDriver,
    humidity: &'static HumidityDriver,
    ambient_light: &'static crate::capsules_extra::ambient_light::AmbientLight<'static>,
    adc: &'static crate::capsules_core::adc::AdcDedicated<'static, crate::sam4l::adc::Adc<'static>>,
    led: &'static crate::capsules_core::led::LedDriver<
        'static,
        LedHigh<'static, crate::sam4l::gpio::GPIOPin<'static>>,
        1,
    >,
    button: &'static crate::capsules_core::button::Button<'static, crate::sam4l::gpio::GPIOPin<'static>>,
    rng: &'static RngDriver,
    analog_comparator: &'static crate::capsules_extra::analog_comparator::AnalogComparator<
        'static,
        crate::sam4l::acifc::Acifc<'static>,
    >,
    spi: &'static crate::capsules_core::spi_controller::Spi<
        'static,
        VirtualSpiMasterDevice<'static, crate::sam4l::spi::SpiHw<'static>>,
    >,
    ipc: crate::kernel::ipc::IPC<{ NUM_PROCS as u8 }>,
    ninedof: &'static crate::capsules_extra::ninedof::NineDof<'static>,
    udp_driver: &'static crate::capsules_extra::net::udp::UDPDriver<'static>,
    crc: &'static crate::capsules_extra::crc::CrcDriver<'static, crate::sam4l::crccu::Crccu<'static>>,
    usb_driver: &'static crate::capsules_extra::usb::usb_user::UsbSyscallDriver<
        'static,
        crate::capsules_extra::usb::usbc_client::Client<'static, crate::sam4l::usbc::Usbc<'static>>,
    >,
    nrf51822: &'static crate::capsules_extra::nrf51822_serialization::Nrf51822Serialization<'static>,
    nonvolatile_storage:
        &'static crate::capsules_extra::nonvolatile_storage_driver::NonvolatileStorage<'static>,
    scheduler: &'static SchedulerInUse,
    systick: crate::cortexm4::systick::SysTick,
}

impl SyscallDriverLookup for Imix {
    fn with_driver<F, R>(&self, driver_num: usize, f: F) -> R
    where
        F: FnOnce(Option<&dyn crate::kernel::syscall::SyscallDriver>) -> R,
    {
        match driver_num {
            crate::capsules_core::console_ordered::DRIVER_NUM => f(Some(self.console)),
            crate::capsules_core::gpio::DRIVER_NUM => f(Some(self.gpio)),
            crate::capsules_core::alarm::DRIVER_NUM => f(Some(self.alarm)),
            crate::capsules_core::spi_controller::DRIVER_NUM => f(Some(self.spi)),
            crate::capsules_core::adc::DRIVER_NUM => f(Some(self.adc)),
            crate::capsules_core::led::DRIVER_NUM => f(Some(self.led)),
            crate::capsules_core::button::DRIVER_NUM => f(Some(self.button)),
            crate::capsules_extra::analog_comparator::DRIVER_NUM => f(Some(self.analog_comparator)),
            crate::capsules_extra::ambient_light::DRIVER_NUM => f(Some(self.ambient_light)),
            crate::capsules_extra::temperature::DRIVER_NUM => f(Some(self.temp)),
            crate::capsules_extra::humidity::DRIVER_NUM => f(Some(self.humidity)),
            crate::capsules_extra::ninedof::DRIVER_NUM => f(Some(self.ninedof)),
            crate::capsules_extra::crc::DRIVER_NUM => f(Some(self.crc)),
            crate::capsules_extra::usb::usb_user::DRIVER_NUM => f(Some(self.usb_driver)),
            crate::capsules_extra::net::udp::DRIVER_NUM => f(Some(self.udp_driver)),
            crate::capsules_extra::nrf51822_serialization::DRIVER_NUM => f(Some(self.nrf51822)),
            crate::capsules_extra::nonvolatile_storage_driver::DRIVER_NUM => {
                f(Some(self.nonvolatile_storage))
            }
            crate::capsules_core::rng::DRIVER_NUM => f(Some(self.rng)),
            crate::kernel::ipc::DRIVER_NUM => f(Some(&self.ipc)),
            _ => f(None),
        }
    }
}

impl KernelResources<crate::sam4l::chip::Sam4l<Sam4lDefaultPeripherals>> for Imix {
    type SyscallDriverLookup = Self;
    type SyscallFilter = ();
    type ProcessFault = ();
    type Scheduler = SchedulerInUse;
    type SchedulerTimer = crate::cortexm4::systick::SysTick;
    type WatchDog = ();
    type ContextSwitchCallback = ();

    fn syscall_driver_lookup(&self) -> &Self::SyscallDriverLookup {
        self
    }
    fn syscall_filter(&self) -> &Self::SyscallFilter {
        &()
    }
    fn process_fault(&self) -> &Self::ProcessFault {
        &()
    }
    fn scheduler(&self) -> &Self::Scheduler {
        self.scheduler
    }
    fn scheduler_timer(&self) -> &Self::SchedulerTimer {
        &self.systick
    }
    fn watchdog(&self) -> &Self::WatchDog {
        &()
    }
    fn context_switch_callback(&self) -> &Self::ContextSwitchCallback {
        &()
    }
}

unsafe fn set_pin_primary_functions(peripherals: &Sam4lDefaultPeripherals) {
    use crate::sam4l::gpio::PeripheralFunction::{A, B, C, E};

    // Right column: Imix pin name
    // Left  column: SAM4L peripheral function
    peripherals.pa[04].configure(Some(A)); // AD0         --  ADCIFE AD0
    peripherals.pa[05].configure(Some(A)); // AD1         --  ADCIFE AD1
    peripherals.pa[06].configure(Some(C)); // EXTINT1     --  EIC EXTINT1
    peripherals.pa[07].configure(Some(A)); // AD1         --  ADCIFE AD2
    peripherals.pa[08].configure(None); //... RF233 IRQ   --  GPIO pin
    peripherals.pa[09].configure(None); //... RF233 RST   --  GPIO pin
    peripherals.pa[10].configure(None); //... RF233 SLP   --  GPIO pin
    peripherals.pa[13].configure(None); //... TRNG EN     --  GPIO pin
    peripherals.pa[14].configure(None); //... TRNG_OUT    --  GPIO pin
    peripherals.pa[17].configure(None); //... NRF INT     -- GPIO pin
    peripherals.pa[18].configure(Some(A)); // NRF CLK     -- USART2_CLK
    peripherals.pa[20].configure(None); //... D8          -- GPIO pin
    peripherals.pa[21].configure(Some(E)); // TWI2 SDA    -- TWIM2_SDA
    peripherals.pa[22].configure(Some(E)); // TWI2 SCL    --  TWIM2 TWCK
    peripherals.pa[25].configure(Some(A)); // USB_N       --  USB DM
    peripherals.pa[26].configure(Some(A)); // USB_P       --  USB DP
    peripherals.pb[00].configure(Some(A)); // TWI1_SDA    --  TWIMS1 TWD
    peripherals.pb[01].configure(Some(A)); // TWI1_SCL    --  TWIMS1 TWCK
    peripherals.pb[02].configure(Some(A)); // AD3         --  ADCIFE AD3
    peripherals.pb[03].configure(Some(A)); // AD4         --  ADCIFE AD4
    peripherals.pb[04].configure(Some(A)); // AD5         --  ADCIFE AD5
    peripherals.pb[05].configure(Some(A)); // VHIGHSAMPLE --  ADCIFE AD6
    peripherals.pb[06].configure(Some(A)); // RTS3        --  USART3 RTS
    peripherals.pb[07].configure(None); //... NRF RESET   --  GPIO
    peripherals.pb[09].configure(Some(A)); // RX3         --  USART3 RX
    peripherals.pb[10].configure(Some(A)); // TX3         --  USART3 TX
    peripherals.pb[11].configure(Some(A)); // CTS0        --  USART0 CTS
    peripherals.pb[12].configure(Some(A)); // RTS0        --  USART0 RTS
    peripherals.pb[13].configure(Some(A)); // CLK0        --  USART0 CLK
    peripherals.pb[14].configure(Some(A)); // RX0         --  USART0 RX
    peripherals.pb[15].configure(Some(A)); // TX0         --  USART0 TX
    peripherals.pc[00].configure(Some(A)); // CS2         --  SPI Nperipherals.pcS2
    peripherals.pc[01].configure(Some(A)); // CS3 (RF233) --  SPI Nperipherals.pcS3
    peripherals.pc[02].configure(Some(A)); // CS1         --  SPI Nperipherals.pcS1
    peripherals.pc[03].configure(Some(A)); // CS0         --  SPI Nperipherals.pcS0
    peripherals.pc[04].configure(Some(A)); // MISO        --  SPI MISO
    peripherals.pc[05].configure(Some(A)); // MOSI        --  SPI MOSI
    peripherals.pc[06].configure(Some(A)); // SCK         --  SPI CLK
    peripherals.pc[07].configure(Some(B)); // RTS2 (BLE)  -- USART2_RTS
    peripherals.pc[08].configure(Some(E)); // CTS2 (BLE)  -- USART2_CTS
    //peripherals.pc[09].configure(None); //... NRF GPIO    -- GPIO
    //peripherals.pc[10].configure(None); //... USER LED    -- GPIO
    peripherals.pc[09].configure(Some(E)); // ACAN1       -- ACIFC comparator
    peripherals.pc[10].configure(Some(E)); // ACAP1       -- ACIFC comparator
    peripherals.pc[11].configure(Some(B)); // RX2 (BLE)   -- USART2_RX
    peripherals.pc[12].configure(Some(B)); // TX2 (BLE)   -- USART2_TX
    //peripherals.pc[13].configure(None); //... ACC_INT1    -- GPIO
    //peripherals.pc[14].configure(None); //... ACC_INT2    -- GPIO
    peripherals.pc[13].configure(Some(E)); //... ACBN1    -- ACIFC comparator
    peripherals.pc[14].configure(Some(E)); //... ACBP1    -- ACIFC comparator
    peripherals.pc[16].configure(None); //... SENSE_PWR   --  GPIO pin
    peripherals.pc[17].configure(None); //... NRF_PWR     --  GPIO pin
    peripherals.pc[18].configure(None); //... RF233_PWR   --  GPIO pin
    peripherals.pc[19].configure(None); //... TRNG_PWR    -- GPIO Pin
    peripherals.pc[22].configure(None); //... KERNEL LED  -- GPIO Pin
    peripherals.pc[24].configure(None); //... USER_BTN    -- GPIO Pin
    peripherals.pc[25].configure(Some(B)); // LI_INT      --  EIC EXTINT2
    peripherals.pc[26].configure(None); //... D7          -- GPIO Pin
    peripherals.pc[27].configure(None); //... D6          -- GPIO Pin
    peripherals.pc[28].configure(None); //... D5          -- GPIO Pin
    peripherals.pc[29].configure(None); //... D4          -- GPIO Pin
    peripherals.pc[30].configure(None); //... D3          -- GPIO Pin
    peripherals.pc[31].configure(None); //... D2          -- GPIO Pin
}

/// This is in a separate, inline(never) function so that its stack frame is
/// removed when this function returns. Otherwise, the stack space used for
/// these static_inits is wasted.
#[inline(never)]
unsafe fn start() -> (
    &'static crate::kernel::Kernel,
    Imix,
    &'static crate::sam4l::chip::Sam4l<Sam4lDefaultPeripherals>,
) {
    ChipHw::init();

    // Initialize deferred calls very early.
    crate::kernel::deferred_call::initialize_deferred_call_state::<
        <ChipHw as crate::kernel::platform::chip::Chip>::ThreadIdProvider,
    >();

    // Bind global variables to this thread.
    let _ = PANIC_RESOURCES
        .bind_to_thread::<<ChipHw as crate::kernel::platform::chip::Chip>::ThreadIdProvider>(
            PanicResources::new(),
        );

    let pm = static_init!(crate::sam4l::pm::PowerManager, crate::sam4l::pm::PowerManager::new());
    let peripherals = static_init!(Sam4lDefaultPeripherals, Sam4lDefaultPeripherals::new(pm));

    pm.setup_system_clock(
        crate::sam4l::pm::SystemClockSource::PllExternalOscillatorAt48MHz {
            frequency: crate::sam4l::pm::OscillatorFrequency::Frequency16MHz,
            startup_mode: crate::sam4l::pm::OscillatorStartup::FastStart,
        },
        &peripherals.flash_controller,
    );

    // Source 32Khz and 1Khz clocks from RC23K (SAM4L Datasheet 11.6.8)
    crate::sam4l::bpm::set_ck32source(crate::sam4l::bpm::CK32Source::RC32K);

    set_pin_primary_functions(peripherals);

    peripherals.setup_circular_deps();
    let chip = static_init!(
        crate::sam4l::chip::Sam4l<Sam4lDefaultPeripherals>,
        crate::sam4l::chip::Sam4l::new(pm, peripherals)
    );
    CHIP = Some(chip);
    PANIC_RESOURCES.get().map(|resources| {
        resources.chip.put(chip);
    });

    // Create capabilities that the board needs to call certain protected kernel
    // functions.
    let grant_cap = create_capability!(capabilities::MemoryAllocationCapability);

    power::configure_submodules(
        &peripherals.pa,
        &peripherals.pb,
        &peripherals.pc,
        power::SubmoduleConfig {
            rf233: true,
            nrf51422: true,
            sensors: true,
            trng: true,
        },
    );

    // Create an array to hold process references.
    let processes = crate::components::process_array::ProcessArrayComponent::new()
        .finalize(crate::components::process_array_component_static!(NUM_PROCS));
    PANIC_RESOURCES.get().map(|resources| {
        resources.processes.put(processes.as_slice());
    });

    // Setup space to store the core kernel data structure.
    let board_kernel = static_init!(crate::kernel::Kernel, crate::kernel::Kernel::new(processes.as_slice()));

    let process_printer = crate::components::process_printer::ProcessPrinterTextComponent::new()
        .finalize(crate::components::process_printer_text_component_static!());
    PANIC_RESOURCES.get().map(|resources| {
        resources.printer.put(process_printer);
    });

    // # CONSOLE
    // Create a shared UART channel for the consoles and for kernel debug.
    peripherals.usart3.set_mode(crate::sam4l::usart::UsartMode::Uart);
    let uart_mux = UartMuxComponent::new(&peripherals.usart3, 115200)
        .finalize(crate::components::uart_mux_component_static!());

    // # TIMER
    let mux_alarm = AlarmMuxComponent::new(&peripherals.ast)
        .finalize(crate::components::alarm_mux_component_static!(crate::sam4l::ast::Ast));
    peripherals.ast.configure(mux_alarm);

    let alarm = AlarmDriverComponent::new(
        board_kernel,
        crate::capsules_core::alarm::DRIVER_NUM,
        mux_alarm,
        create_capability!(capabilities::MemoryAllocationCapability),
    )
    .finalize(crate::components::alarm_component_static!(crate::sam4l::ast::Ast));

    let process_console_cap = unsafe { crate::kernel::mint_defined_capability!(ProcessConsoleCap) };
    let pconsole = ProcessConsoleComponent::new(
        board_kernel,
        uart_mux,
        mux_alarm,
        process_printer,
        Some(crate::cortexm4::support::reset),
        process_console_cap,
    )
    .finalize(crate::components::process_console_component_static!(
        crate::sam4l::ast::Ast,
        ProcessConsoleCap
    ));

    let console = ConsoleOrderedComponent::new(
        board_kernel,
        crate::capsules_core::console_ordered::DRIVER_NUM,
        uart_mux,
        mux_alarm,
        200,
        5,
        5,
        create_capability!(capabilities::MemoryAllocationCapability),
    )
    .finalize(crate::components::console_ordered_component_static!(
        crate::sam4l::ast::Ast
    ));
    DebugWriterComponent::new::<<ChipHw as crate::kernel::platform::chip::Chip>::ThreadIdProvider>(
        uart_mux,
        create_capability!(capabilities::SetDebugWriterCapability),
    )
    .finalize(crate::components::debug_writer_component_static!());

    // Allow processes to communicate over BLE through the nRF51822
    peripherals.usart2.set_mode(crate::sam4l::usart::UsartMode::Uart);
    let nrf_serialization = Nrf51822Component::new(
        board_kernel,
        crate::capsules_extra::nrf51822_serialization::DRIVER_NUM,
        &peripherals.usart2,
        &peripherals.pb[07],
        create_capability!(capabilities::MemoryAllocationCapability),
    )
    .finalize(crate::components::nrf51822_component_static!());

    // # I2C and I2C Sensors
    let mux_i2c = static_init!(
        MuxI2C<'static, crate::sam4l::i2c::I2CHw<'static>>,
        MuxI2C::new(&peripherals.i2c2, None)
    );
    crate::kernel::deferred_call::DeferredCallClient::register(mux_i2c);
    peripherals.i2c2.set_master_client(mux_i2c);

    let isl29035 = Isl29035Component::new(mux_i2c, mux_alarm).finalize(
        crate::components::isl29035_component_static!(crate::sam4l::ast::Ast, crate::sam4l::i2c::I2CHw<'static>),
    );
    let ambient_light = AmbientLightComponent::new(
        board_kernel,
        crate::capsules_extra::ambient_light::DRIVER_NUM,
        isl29035,
        create_capability!(capabilities::MemoryAllocationCapability),
    )
    .finalize(crate::components::ambient_light_component_static!());

    let si7021 = SI7021Component::new(mux_i2c, mux_alarm, 0x40).finalize(
        crate::components::si7021_component_static!(crate::sam4l::ast::Ast, crate::sam4l::i2c::I2CHw<'static>),
    );
    let temp = crate::components::temperature::TemperatureComponent::new(
        board_kernel,
        crate::capsules_extra::temperature::DRIVER_NUM,
        si7021,
        create_capability!(capabilities::MemoryAllocationCapability),
    )
    .finalize(crate::components::temperature_component_static!(SI7021Sensor));
    let humidity = crate::components::humidity::HumidityComponent::new(
        board_kernel,
        crate::capsules_extra::humidity::DRIVER_NUM,
        si7021,
        create_capability!(capabilities::MemoryAllocationCapability),
    )
    .finalize(crate::components::humidity_component_static!(SI7021Sensor));

    let fxos8700 = crate::components::fxos8700::Fxos8700Component::new(mux_i2c, 0x1e, &peripherals.pc[13])
        .finalize(crate::components::fxos8700_component_static!(
            crate::sam4l::i2c::I2CHw<'static>
        ));

    let ninedof = crate::components::ninedof::NineDofComponent::new(
        board_kernel,
        crate::capsules_extra::ninedof::DRIVER_NUM,
        create_capability!(capabilities::MemoryAllocationCapability),
    )
    .finalize(crate::components::ninedof_component_static!(fxos8700));

    // SPI MUX, SPI syscall driver and RF233 radio
    let mux_spi = crate::components::spi::SpiMuxComponent::new(&peripherals.spi).finalize(
        crate::components::spi_mux_component_static!(crate::sam4l::spi::SpiHw<'static>),
    );

    let spi_syscalls = SpiSyscallComponent::new(
        board_kernel,
        mux_spi,
        crate::sam4l::spi::Peripheral::Peripheral2,
        crate::capsules_core::spi_controller::DRIVER_NUM,
        create_capability!(capabilities::MemoryAllocationCapability),
    )
    .finalize(crate::components::spi_syscall_component_static!(
        crate::sam4l::spi::SpiHw<'static>
    ));
    let rf233_spi = SpiComponent::new(mux_spi, crate::sam4l::spi::Peripheral::Peripheral3).finalize(
        crate::components::spi_component_static!(crate::sam4l::spi::SpiHw<'static>),
    );
    let rf233 = crate::components::rf233::RF233Component::new(
        rf233_spi,
        &peripherals.pa[09], // reset
        &peripherals.pa[10], // sleep
        &peripherals.pa[08], // irq
        &peripherals.pa[08],
        RADIO_CHANNEL,
    )
    .finalize(crate::components::rf233_component_static!(
        crate::sam4l::spi::SpiHw<'static>
    ));

    // Setup ADC
    let adc_channels = static_init!(
        [crate::sam4l::adc::AdcChannel; 6],
        [
            crate::sam4l::adc::AdcChannel::new(crate::sam4l::adc::Channel::AD1), // AD0
            crate::sam4l::adc::AdcChannel::new(crate::sam4l::adc::Channel::AD2), // AD1
            crate::sam4l::adc::AdcChannel::new(crate::sam4l::adc::Channel::AD3), // AD2
            crate::sam4l::adc::AdcChannel::new(crate::sam4l::adc::Channel::AD4), // AD3
            crate::sam4l::adc::AdcChannel::new(crate::sam4l::adc::Channel::AD5), // AD4
            crate::sam4l::adc::AdcChannel::new(crate::sam4l::adc::Channel::AD6), // AD5
        ]
    );
    let adc = crate::components::adc::AdcDedicatedComponent::new(
        &peripherals.adc,
        adc_channels,
        board_kernel,
        crate::capsules_core::adc::DRIVER_NUM,
        create_capability!(capabilities::MemoryAllocationCapability),
    )
    .finalize(crate::components::adc_dedicated_component_static!(crate::sam4l::adc::Adc));

    let gpio = GpioComponent::new(
        board_kernel,
        crate::capsules_core::gpio::DRIVER_NUM,
        crate::components::gpio_component_helper!(
            crate::sam4l::gpio::GPIOPin,
            0 => &peripherals.pc[31],
            1 => &peripherals.pc[30],
            2 => &peripherals.pc[29],
            3 => &peripherals.pc[28],
            4 => &peripherals.pc[27],
            5 => &peripherals.pc[26],
            6 => &peripherals.pa[20]
        ),
        create_capability!(capabilities::MemoryAllocationCapability),
    )
    .finalize(crate::components::gpio_component_static!(crate::sam4l::gpio::GPIOPin));

    let led = LedsComponent::new().finalize(crate::components::led_component_static!(
        LedHigh<'static, crate::sam4l::gpio::GPIOPin>,
        LedHigh::new(&peripherals.pc[10]),
    ));

    let button = crate::components::button::ButtonComponent::new(
        board_kernel,
        crate::capsules_core::button::DRIVER_NUM,
        crate::components::button_component_helper!(
            crate::sam4l::gpio::GPIOPin,
            (
                &peripherals.pc[24],
                crate::kernel::hil::gpio::ActivationMode::ActiveLow,
                crate::kernel::hil::gpio::FloatingState::PullNone
            )
        ),
        create_capability!(capabilities::MemoryAllocationCapability),
    )
    .finalize(crate::components::button_component_static!(crate::sam4l::gpio::GPIOPin));

    let crc = CrcComponent::new(
        board_kernel,
        crate::capsules_extra::crc::DRIVER_NUM,
        &peripherals.crccu,
        create_capability!(capabilities::MemoryAllocationCapability),
    )
    .finalize(crate::components::crc_component_static!(crate::sam4l::crccu::Crccu));

    let analog_comparator = crate::components::analog_comparator::AnalogComparatorComponent::new(
        &peripherals.acifc,
        crate::components::analog_comparator_component_helper!(
            <crate::sam4l::acifc::Acifc as crate::kernel::hil::analog_comparator::AnalogComparator>::Channel,
            crate::sam4l::acifc::AcChannel::new(crate::sam4l::acifc::Channel::AC0),
            crate::sam4l::acifc::AcChannel::new(crate::sam4l::acifc::Channel::AC0),
            crate::sam4l::acifc::AcChannel::new(crate::sam4l::acifc::Channel::AC0),
            crate::sam4l::acifc::AcChannel::new(crate::sam4l::acifc::Channel::AC0),
        ),
        board_kernel,
        crate::capsules_extra::analog_comparator::DRIVER_NUM,
        create_capability!(capabilities::MemoryAllocationCapability),
    )
    .finalize(crate::components::analog_comparator_component_static!(
        crate::sam4l::acifc::Acifc
    ));
    let rng = RngComponent::new(
        board_kernel,
        crate::capsules_core::rng::DRIVER_NUM,
        &peripherals.trng,
        create_capability!(capabilities::MemoryAllocationCapability),
    )
    .finalize(crate::components::rng_component_static!(crate::sam4l::trng::Trng));

    // For now, assign the 802.15.4 MAC address on the device as
    // simply a 16-bit short address which represents the last 16 bits
    // of the serial number of the sam4l for this device.  In the
    // future, we could generate the DEFAULT_EXT_SRC_MAC address by hashing the full
    // 120-bit serial number
    let serial_num: crate::sam4l::serial_num::SerialNum = crate::sam4l::serial_num::SerialNum::new();
    let serial_num_bottom_16 = (serial_num.get_lower_64() & 0x0000_0000_0000_ffff) as u16;
    let src_mac_from_serial_num: MacAddress = MacAddress::Short(serial_num_bottom_16);
    const DEFAULT_EXT_SRC_MAC: [u8; 8] = [0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77];

    let aes_mux = static_init!(
        MuxAES128CCM<'static, crate::sam4l::aes::Aes>,
        MuxAES128CCM::new(&peripherals.aes)
    );
    aes_mux.register();
    peripherals.aes.set_client(aes_mux);

    let (_, mux_mac) = crate::components::ieee802154::Ieee802154Component::new(
        board_kernel,
        crate::capsules_extra::ieee802154::DRIVER_NUM,
        rf233,
        aes_mux,
        PAN_ID,
        serial_num_bottom_16,
        DEFAULT_EXT_SRC_MAC,
        create_capability!(capabilities::MemoryAllocationCapability),
    )
    .finalize(crate::components::ieee802154_component_static!(
        crate::capsules_extra::rf233::RF233<
            'static,
            VirtualSpiMasterDevice<'static, crate::sam4l::spi::SpiHw<'static>>,
        >,
        crate::sam4l::aes::Aes<'static>
    ));

    let usb_driver = crate::components::usb::UsbComponent::new(
        board_kernel,
        crate::capsules_extra::usb::usb_user::DRIVER_NUM,
        &peripherals.usbc,
        create_capability!(capabilities::MemoryAllocationCapability),
    )
    .finalize(crate::components::usb_component_static!(crate::sam4l::usbc::Usbc));

    // Kernel storage region, allocated with the storage_volume!
    // macro in common/utils.rs
    extern "C" {
        /// Beginning on the ROM region containing app images.
        static _sstorage: u8;
        static _estorage: u8;
    }

    let nonvolatile_storage = crate::components::nonvolatile_storage::NonvolatileStorageComponent::new(
        board_kernel,
        crate::capsules_extra::nonvolatile_storage_driver::DRIVER_NUM,
        &peripherals.flash_controller,
        0x60000, // Start address for userspace accessible region
        0x20000, // Length of userspace accessible region
        core::ptr::addr_of!(_sstorage) as usize, //start address of kernel region
        core::ptr::addr_of!(_estorage) as usize - core::ptr::addr_of!(_sstorage) as usize, // length of kernel region
        create_capability!(capabilities::MemoryAllocationCapability),
    )
    .finalize(crate::components::nonvolatile_storage_component_static!(
        crate::sam4l::flashcalw::FLASHCALW
    ));

    let local_ip_ifaces = static_init!(
        [IPAddr; 3],
        [
            IPAddr([
                0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
                0x0e, 0x0f,
            ]),
            IPAddr([
                0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d,
                0x1e, 0x1f,
            ]),
            IPAddr::generate_from_mac(src_mac_from_serial_num),
        ]
    );

    let (udp_send_mux, udp_recv_mux, udp_port_table) = crate::components::udp_mux::UDPMuxComponent::new(
        mux_mac,
        DEFAULT_CTX_PREFIX_LEN,
        DEFAULT_CTX_PREFIX,
        DST_MAC_ADDR,
        src_mac_from_serial_num, //comment out for dual rx test only
        //MacAddress::Short(49138), //comment in for dual rx test only
        local_ip_ifaces,
        mux_alarm,
        create_capability!(capabilities::NetworkCapabilityCreationCapability),
        create_capability!(capabilities::CreatePortTableCapability),
    )
    .finalize(crate::components::udp_mux_component_static!(
        crate::sam4l::ast::Ast,
        Ieee802154MacDevice
    ));

    // UDP driver initialization happens here
    crate::kernel::create_typed_capability!(udp_driver_cap, UdpDriverCap: crate::kernel::capabilities::UdpDriverCapability);
    let udp_driver = crate::components::udp_driver::UDPDriverComponent::new(
        board_kernel,
        crate::capsules_extra::net::udp::driver::DRIVER_NUM,
        udp_send_mux,
        udp_recv_mux,
        udp_port_table,
        local_ip_ifaces,
        udp_driver_cap,
        create_capability!(capabilities::MemoryAllocationCapability),
        create_capability!(capabilities::NetworkCapabilityCreationCapability),
    )
    .finalize(crate::components::udp_driver_component_static!(
        crate::sam4l::ast::Ast,
        UdpDriverCap
    ));

    let scheduler = crate::components::sched::round_robin::RoundRobinComponent::new(processes)
        .finalize(crate::components::round_robin_component_static!(NUM_PROCS));

    // Create the software-based SHA engine.
    let sha = crate::components::sha::ShaSoftware256Component::new()
        .finalize(crate::components::sha_software_256_component_static!());

    // Create the credential checker.
    let checking_policy = crate::components::appid::checker_sha::AppCheckerSha256Component::new(sha)
        .finalize(crate::components::app_checker_sha256_component_static!());

    // Create the AppID assigner.
    let assigner = crate::components::appid::assigner_name::AppIdAssignerNamesComponent::new()
        .finalize(crate::components::appid_assigner_names_component_static!());

    // Create the process checking machine.
    let checker = crate::components::appid::checker::ProcessCheckerMachineComponent::new(checking_policy)
        .finalize(crate::components::process_checker_machine_component_static!());

    //--------------------------------------------------------------------------
    // STORAGE PERMISSIONS
    //--------------------------------------------------------------------------

    crate::kernel::create_typed_capability!(app_store_cap, AppStoreCap: crate::kernel::capabilities::ApplicationStorageCapability);
    let storage_permissions_policy =
        crate::components::storage_permissions::individual::StoragePermissionsIndividualComponent::new(
            app_store_cap,
        )
        .finalize(
            crate::components::storage_permissions_individual_component_static!(
                crate::sam4l::chip::Sam4l<Sam4lDefaultPeripherals>,
                crate::kernel::process::ProcessStandardDebugFull,
                AppStoreCap,
            ),
        );

    //--------------------------------------------------------------------------
    // PROCESS LOADING
    //--------------------------------------------------------------------------

    // These symbols are defined in the standard Tock linker script.
    extern "C" {
        /// Beginning of the ROM region containing app images.
        static _sapps: u8;
        /// End of the ROM region containing app images.
        static _eapps: u8;
        /// Beginning of the RAM region for app memory.
        static mut _sappmem: u8;
        /// End of the RAM region for app memory.
        static _eappmem: u8;
    }

    let app_flash = core::slice::from_raw_parts(
        core::ptr::addr_of!(_sapps),
        core::ptr::addr_of!(_eapps) as usize - core::ptr::addr_of!(_sapps) as usize,
    );
    let app_memory = core::slice::from_raw_parts_mut(
        core::ptr::addr_of_mut!(_sappmem),
        core::ptr::addr_of!(_eappmem) as usize - core::ptr::addr_of!(_sappmem) as usize,
    );

    // Create and start the asynchronous process loader.
    let _loader = crate::components::loader::sequential::ProcessLoaderSequentialComponent::new(
        checker,
        board_kernel,
        chip,
        &FAULT_RESPONSE,
        assigner,
        storage_permissions_policy,
        app_flash,
        app_memory,
        create_capability!(capabilities::ProcessManagementCapability),
    )
    .finalize(crate::components::process_loader_sequential_component_static!(
        crate::sam4l::chip::Sam4l<Sam4lDefaultPeripherals>,
        crate::kernel::process::ProcessStandardDebugFull,
        NUM_PROCS
    ));

    let imix = Imix {
        pconsole,
        console,
        alarm,
        gpio,
        temp,
        humidity,
        ambient_light,
        adc,
        led,
        button,
        rng,
        analog_comparator,
        crc,
        spi: spi_syscalls,
        ipc: crate::kernel::ipc::IPC::new(board_kernel, crate::kernel::ipc::DRIVER_NUM, &grant_cap),
        ninedof,
        udp_driver,
        usb_driver,
        nrf51822: nrf_serialization,
        nonvolatile_storage,
        scheduler,
        systick: crate::cortexm4::systick::SysTick::new(),
    };

    // Need to initialize the UART for the nRF51 serialization.
    imix.nrf51822.initialize();

    // These two lines need to be below the creation of the chip for
    // initialization to work.
    let _ = rf233.reset();
    let _ = rf233.start();

    let _ = imix.pconsole.start();

    // Optional kernel tests. Note that these might conflict
    // with normal operation (e.g., steal callbacks from drivers, etc.),
    // so do not run these and expect all services/applications to work.
    // Once everything is virtualized in the kernel this won't be a problem.
    // -pal, 11/20/18
    //
    //test::virtual_uart_rx_test::run_virtual_uart_receive(uart_mux);
    //test::rng_test::run_entropy32(&peripherals.trng);
    //test::virtual_aes_ccm_test::run(&peripherals.aes);
    //test::aes_test::run_aes128_ctr(&peripherals.aes);
    //test::aes_test::run_aes128_cbc(&peripherals.aes);
    //test::log_test::run(
    //    mux_alarm,
    //    &peripherals.flash_controller,
    //);
    //test::linear_log_test::run(
    //    mux_alarm,
    //    &peripherals.flash_controller,
    //);
    //test::icmp_lowpan_test::run(mux_mac, mux_alarm);
    //let lowpan_frag_test = test::ipv6_lowpan_test::initialize_all(mux_mac, mux_alarm);
    //lowpan_frag_test.start(); // If flashing the transmitting Imix
    /*let udp_lowpan_test = test::udp_lowpan_test::initialize_all(
       udp_send_mux,
        udp_recv_mux,
        udp_port_table,
        mux_alarm,
    );*/
    //udp_lowpan_test.start();

    // alarm_test::run_alarm(&peripherals.ast);
    /*let virtual_alarm_timer = static_init!(
        VirtualMuxAlarm<'static, crate::sam4l::ast::Ast>,
        VirtualMuxAlarm::new(mux_alarm)
    );
    virtual_alarm_timer.setup();

    let mux_timer = static_init!(
        MuxTimer<'static, crate::sam4l::ast::Ast>,
        MuxTimer::new(virtual_alarm_timer)
    );*/
    //virtual_alarm_timer.set_alarm_client(mux_timer);

    //test::sha256_test::run_sha256();

    /*crate::components::test::multi_alarm_test::MultiAlarmTestComponent::new(mux_alarm)
    .finalize(crate::components::multi_alarm_test_component_buf!(crate::sam4l::ast::Ast))
    .run();*/

    debug!("Initialization complete. Entering main loop");

    (board_kernel, imix, chip)
}

/// Main function called after RAM initialized.
#[no_mangle]
pub unsafe fn main() {
    let main_loop_capability = create_capability!(capabilities::MainLoopCapability);

    let (board_kernel, platform, chip) = start();
    board_kernel.kernel_loop(&platform, chip, Some(&platform.ipc), &main_loop_capability);
}
