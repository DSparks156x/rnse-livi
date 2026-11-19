// SPDX-FileCopyrightText: 2026 Drew Sparks
// SPDX-License-Identifier: GPL-3.0-or-later

//! The default panels and log profiles, drawn like `shots`:
//! strip where LIVI's tab bar goes:
//!
//! ```text
//! cargo run -p livi-carpi-ui --features shots --example pitch -- <dir>
//! ```

use std::cell::Cell as StdCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use carpi_proto::{Reading, Value};
use livi_carpi_ui::client::Fixture;
use livi_carpi_ui::view::{Hit, View};
use livi_carpi_ui::{Client, PageId, Pages, config::Saved};
use slint::platform::software_renderer::{MinimalSoftwareWindow, RepaintBufferType, Rgb565Pixel};
use slint::platform::{Platform, WindowAdapter};
use slint::{ComponentHandle, PhysicalSize};

slint::slint! {
    import "../../livi-ui/ui/fonts/Roboto-400.ttf";
    import "../../livi-ui/ui/fonts/Roboto-500.ttf";
    import "../../livi-ui/ui/fonts/Roboto-700.ttf";
    import { Theme } from "../../livi-ui/ui/theme.slint";
    import { Focus, FocusZone } from "../../livi-ui/ui/focus.slint";
    import { CarpiView, CarpiState, CarpiLayout, CarpiKind, CarpiPill, CarpiRow, CarpiCell, CarpiSpot, CarpiPoint, CarpiTick, CarpiToast, CarpiToastView, CarpiKeyboard, CarpiKey, CarpiKeyRow, CarpiWidget, CarpiLine, CarpiItem, CarpiMark, CarpiEmpty } from "../ui/carpi.slint";
    export { Theme, Focus, FocusZone, CarpiState, CarpiLayout, CarpiKind, CarpiPill, CarpiRow, CarpiCell, CarpiSpot, CarpiPoint, CarpiTick, CarpiToast, CarpiKeyboard, CarpiKey, CarpiKeyRow, CarpiWidget, CarpiLine, CarpiItem, CarpiMark, CarpiEmpty }

    export component Shot inherits Window {
        width: 800px;
        height: 480px;
        default-font-family: "Roboto";
        background: Theme.background;
        // Where LIVI's tab bar sits.
        Rectangle {
            x: 0;
            width: 56px;
            background: Theme.background.mix(Theme.dark ? #ffffff : #000000, 0.88);
        }
        CarpiView {
            x: 56px;
            width: 744px;
            height: 480px;
        }
        CarpiToastView {
            window-width: 744px;
            x: 56px + (744px - self.width) / 2;
            y: 480px - self.height - 20px;
        }
    }
}

include!("../ui/glue.rs");

thread_local! {
    static NOW: StdCell<Duration> = const { StdCell::new(Duration::ZERO) };
}

struct Offscreen(Rc<MinimalSoftwareWindow>);

impl Platform for Offscreen {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, slint::PlatformError> {
        Ok(self.0.clone())
    }
    fn duration_since_start(&self) -> Duration {
        NOW.get()
    }
}

struct Shots {
    window: Rc<MinimalSoftwareWindow>,
    ui: Shot,
    models: Models,
    dir: std::path::PathBuf,
}

impl Shots {
    fn show(&self, view: &View, focus: i32, dark: bool) {
        self.ui.global::<Theme>().set_dark(dark);
        self.models.fill(&self.ui.global::<CarpiState>(), view);
        let f = self.ui.global::<Focus>();
        f.set_zone(if focus >= 0 {
            FocusZone::Main
        } else {
            FocusZone::None
        });
        f.set_main_index(-1);
        self.settle();
        f.set_main_index(focus);
        self.settle();
    }

    /// Runs animations to their end.
    fn settle(&self) {
        for _ in 0..20 {
            NOW.set(NOW.get() + Duration::from_millis(100));
            slint::platform::update_timers_and_animations();
            self.window.request_redraw();
            self.window.draw_if_needed(|_| {});
        }
    }

    fn save(&self, name: &str) {
        let (w, h) = (800usize, 480usize);
        let mut buf = vec![Rgb565Pixel(0); w * h];
        self.window.request_redraw();
        self.window.draw_if_needed(|r| {
            r.render(&mut buf, w);
        });
        let mut img = image::RgbImage::new(w as u32, h as u32);
        for (i, p) in buf.iter().enumerate() {
            let (r, g, b) = (p.0 >> 11, (p.0 >> 5) & 0x3f, p.0 & 0x1f);
            img.put_pixel(
                (i % w) as u32,
                (i / w) as u32,
                image::Rgb([
                    (r * 255 / 31) as u8,
                    (g * 255 / 63) as u8,
                    (b * 255 / 31) as u8,
                ]),
            );
        }
        let path = self.dir.join(format!("{name}.png"));
        img.save(&path).expect("png");
        println!("{}", path.display());
    }
}

/// The fixture, with some diag values as carpid would poll them.
fn client() -> Client {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/car.json");
    let client = Fixture::read(&path).expect("fixture").start();
    let diag = [
        ("engine.boost.actual", 1840.0),
        ("engine.boost.specified", 1900.0),
        ("engine.load_pct", 62.0),
        ("engine.throttle_pct", 38.0),
        ("engine.intake_temp_c", 31.0),
        ("engine.oil_temp_c", 96.0),
        ("engine.maf_gs", 48.5),
        ("engine.lambda.actual", 0.992),
    ];
    client.snapshot().values.take(
        diag.iter()
            .map(|(n, v)| Reading {
                name: n.to_string(),
                value: Some(Value::Num(*v)),
                age_ms: 0,
            })
            .collect(),
        Instant::now(),
    );
    client
}

fn row(v: &View, label: &str) -> Hit {
    Hit::Row(v.rows.iter().position(|r| r.label == label).expect(label))
}

fn main() {
    let dir = std::env::args()
        .nth(1)
        .map_or_else(|| std::path::PathBuf::from("pitch"), Into::into);
    std::fs::create_dir_all(&dir).expect("dir");
    let window = MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer);
    slint::platform::set_platform(Box::new(Offscreen(window.clone()))).expect("platform");
    let ui = Shot::new().expect("ui");
    window.set_size(PhysicalSize::new(800, 480));
    ui.show().expect("show");
    let shots = Shots {
        window,
        ui,
        models: Models::default(),
        dir,
    };
    shots.models.wire(&shots.ui.global::<CarpiState>());

    let client = client();
    let mut pages = Pages::new(Saved::default(), None);
    let now = Instant::now();
    let view = |p: &Pages| p.view(PageId::Data, &client.snapshot(), now);

    let set = panels();
    let names: Vec<String> = set.panels.iter().map(|p| p.name.clone()).collect();
    let n = set.panels.len();
    pages.set_panels(set);
    // A minute of a spirited drive: a pull every ~18 s.
    for i in 0..600u32 {
        let t = i as f64 / 10.0;
        let pull = (t * 0.35).sin().max(0.0);
        let boost = 1013.0 + 900.0 * ((t * 0.35).sin().max(-0.4) + 0.1);
        let spec = 1013.0 + if (t * 0.35).sin() > 0.2 { 1050.0 } else { 80.0 };
        let rail_spec = 45.0 + 70.0 * pull;
        let vals: Vec<(&str, f64)> = vec![
            ("engine.boost.actual", boost),
            ("engine.boost.specified", spec),
            ("engine.boost.cluster_mbar", boost - 20.0),
            ("engine.altitude_factor", 0.98),
            ("engine.rpm", 2400.0 + 3200.0 * pull),
            ("engine.coolant_c", 91.0),
            ("engine.oil_temp_c", 108.0),
            ("engine.intake_temp_c", 34.0 + 6.0 * pull),
            ("engine.exhaust_temp.projected_c", 620.0 + 260.0 * pull),
            ("engine.n75_duty_pct", 30.0 + 45.0 * pull),
            ("engine.load_pct", 40.0 + 130.0 * pull),
            ("engine.maf_gs", 25.0 + 150.0 * pull),
            ("engine.injection_ms", 1.2 + 3.1 * pull),
            ("engine.fuel_rail.specified_bar", rail_spec),
            (
                "engine.fuel_rail.actual_bar",
                rail_spec - 4.0 * pull + 1.5 * (t * 3.0).sin(),
            ),
            ("engine.fuel_pump_pct", 18.0 + 55.0 * pull),
            ("engine.lambda.specified", 1.0 - 0.2 * pull),
            (
                "engine.lambda.actual",
                1.0 - 0.19 * pull + 0.01 * (t * 5.0).sin(),
            ),
            ("engine.ignition_timing_deg", 22.0 - 14.0 * pull),
            ("engine.timing_retard.cyl1_deg", -1.5 * pull),
            ("engine.timing_retard.cyl2_deg", -4.5 * pull),
            ("engine.timing_retard.cyl3_deg", 0.0),
            ("engine.timing_retard.cyl4_deg", -0.8 * pull),
            ("engine.generator_pct", 46.0 + 4.0 * (t * 0.5).sin()),
            ("awd.plate_temp_c", 61.0),
            ("awd.oil_temp_c", 54.0),
            ("awd.oil_pressure_bar", 6.0 + 22.0 * pull),
            ("awd.estimated_torque_nm", 120.0 + 900.0 * pull),
            ("awd.valve_opening_pct", 70.0 - 55.0 * pull),
            ("trans.k1.pressure_bar", 9.8),
            ("trans.k2.pressure_bar", 0.4),
            ("trans.clutch_oil_temp_c", 96.0),
            ("trans.fluid_temp_c", 88.0),
            ("trans.module_temp_c", 71.0),
            ("trans.selector", 5.0),
            ("power.battery_v", 14.2 + 0.15 * (t * 0.2).sin()),
            ("power.battery_soc_pct", 82.0),
            ("ambient.outside_temp_c", 7.5),
            ("fuel.range_km", 412.0),
            ("fuel.level_l", 38.0),
            ("fuel.consumption_l100km", 8.4),
            ("trip.short.consumption_l100km", 9.6),
            ("trip.short.distance_km", 48.2),
            ("trip.short.speed_kmh", 57.0),
            ("trip.tank.consumption_l100km", 8.9),
            ("engine.fan_pct", 20.0),
            ("engine.throttle_deg", 12.0 + 70.0 * pull),
            ("engine.lambda.bank1.enrichment_factor", 1.0 + 0.2 * pull),
            ("engine.fuel_temp.calculated_c", 41.0),
            ("engine.terminal30_v", 14.0),
            ("trans.k1.shaft_rpm", 2400.0 + 3200.0 * pull),
            ("trans.k2.shaft_rpm", 1700.0 + 2300.0 * pull),
            ("trans.k1.torque_specified_nm", 60.0 + 250.0 * pull),
            ("trans.k2.torque_specified_nm", 0.0),
            ("trans.k1.valve_a", 0.62 + 0.3 * pull),
            ("trans.k2.valve_a", 0.11),
            ("awd.commanded_nm", 150.0 + 950.0 * pull),
            ("awd.slip_nm", 40.0 * pull),
            ("awd.measured_yaw_dps", 4.0 * (t * 0.4).sin()),
            ("awd.model_yaw_dps", 3.6 * (t * 0.4).sin()),
            ("power.generator_target_v", 14.3),
            ("power.bdm_v", 14.1),
            ("power.load_shed_stage", 0.0),
            ("trip.long.consumption_l100km", 9.1),
            ("trip.long.distance_km", 1823.0),
            ("trip.long.speed_kmh", 64.0),
            ("trip.tank.distance_km", 233.0),
            ("trip.short.time_min", 51.0),
            ("diag.02.15.1", 8.2),
            ("diag.02.15.2", 2.0 + 6.0 * pull),
            ("diag.02.15.3", 0.2),
            ("diag.02.15.4", -0.1),
            ("diag.02.17.1", 4.1),
            ("diag.02.17.2", -4.0),
            ("diag.02.17.3", 4.4),
            ("diag.02.17.4", 3.9),
            ("diag.02.18.1", -4.3),
            ("diag.02.18.2", 4.2),
            ("diag.02.18.3", -4.1),
            ("diag.02.18.4", -4.5),
            ("vehicle.speed_kmh", 70.0 + 50.0 * pull),
        ];
        let at = now - Duration::from_millis(100 * (600 - i as u64));
        let mut rs: Vec<Reading> = vals
            .iter()
            .map(|(n, v)| Reading {
                name: n.to_string(),
                value: Some(Value::Num(*v)),
                age_ms: 0,
            })
            .collect();
        rs.push(Reading {
            name: "trans.shifting".into(),
            value: Some(Value::Bool(false)),
            age_ms: 0,
        });
        rs.push(Reading {
            name: "awd.mode".into(),
            value: Some(Value::Text("Performance".into())),
            age_ms: 0,
        });
        client.snapshot().values.take(rs, at);
        if i % 3 == 0 {
            for k in 0..n {
                pages.set_shown(k);
                let _ = pages.view(PageId::Data, &client.snapshot(), at);
            }
        }
    }
    for (k, name) in names.iter().enumerate() {
        pages.set_shown(k);
        let v = view(&pages);
        shots.show(&v, v.start, true);
        shots.save(&format!("panel-{k}-{}", name.to_lowercase()));
    }
    pages.set_shown(0);
    let v = view(&pages);
    shots.show(&v, v.start, false);
    shots.save("panel-0-daily-day");

    // Log profiles.
    client.snapshot().datalog = Some(datalog());
    pages.open(PageId::Data, "logs");
    let v = view(&pages);
    let profiles = row(&v, "Profiles");
    pages.press(PageId::Data, &v, &profiles, 0, &client);
    let v = view(&pages);
    shots.show(&v, 0, true);
    shots.save("log-profiles");
    shots.show(&v, 6, true);
    shots.save("log-profiles-end");
    let r = row(&v, "Pull");
    pages.press(PageId::Data, &v, &r, 0, &client);
    let v = view(&pages);
    shots.show(&v, 1, true);
    shots.save("log-pull");
    let ch = row(&v, "Channels");
    pages.press(PageId::Data, &v, &ch, 2, &client);
    let v = view(&pages);
    shots.show(&v, 1, true);
    shots.save("log-pull-channels");
}

/// The default panels, without Scratch.
fn panels() -> livi_carpi_ui::panels::Panels {
    let mut panels = livi_carpi_ui::panels::defaults();
    panels.retain(|p| !p.scratch);
    livi_carpi_ui::panels::Panels { panels, shown: 0 }
}

/// The pitched log profiles, with what carpid's `datalog::load` says each costs.
fn datalog() -> carpi_proto::DatalogStatus {
    use carpi_proto::{DatalogLoad, DatalogProfile, DatalogProfileStatus};
    type Load = (u32, u32, u32, u32, Option<f64>);
    let p = |name: &str, values: &[&str], rate_hz, l: Load| DatalogProfileStatus {
        profile: DatalogProfile {
            name: name.into(),
            values: values.iter().map(|v| v.to_string()).collect(),
            rate_hz,
            labels: Vec::new(),
        },
        load: DatalogLoad {
            bus_values: l.0,
            diag_values: l.1,
            diag_groups: l.2,
            diag_modules: l.3,
            diag_hz: l.4,
        },
    };
    carpi_proto::DatalogStatus {
        profiles: vec![
            p(
                "Pull",
                &[
                    "engine.rpm",
                    "vehicle.speed_kmh",
                    "engine.throttle_pct",
                    "engine.load_pct",
                    "engine.maf_gs",
                    "engine.boost.actual",
                    "engine.boost.specified",
                    "engine.n75_duty_pct",
                    "engine.intake_temp_c",
                    "engine.ignition_timing_deg",
                ],
                5,
                (2, 8, 3, 1, Some(4.4)),
            ),
            p(
                "Knock",
                &[
                    "engine.rpm",
                    "engine.load_pct",
                    "engine.ignition_timing_deg",
                    "engine.timing_retard.cyl1_deg",
                    "engine.timing_retard.cyl2_deg",
                    "engine.timing_retard.cyl3_deg",
                    "engine.timing_retard.cyl4_deg",
                    "engine.intake_temp_c",
                    "engine.coolant_c",
                ],
                5,
                (2, 7, 3, 1, Some(4.4)),
            ),
            p(
                "Fuel",
                &[
                    "engine.rpm",
                    "engine.load_pct",
                    "engine.maf_gs",
                    "engine.injection_ms",
                    "engine.fuel_rail.actual_bar",
                    "engine.fuel_rail.specified_bar",
                    "engine.fuel_pump_pct",
                    "engine.lambda.actual",
                    "engine.lambda.specified",
                ],
                5,
                (1, 8, 3, 1, Some(4.4)),
            ),
            p(
                "DSG",
                &[
                    "engine.rpm",
                    "vehicle.speed_kmh",
                    "trans.selector",
                    "trans.k1.pressure_bar",
                    "trans.k2.pressure_bar",
                    "trans.k1.shaft_rpm",
                    "trans.k2.shaft_rpm",
                    "trans.k1.torque_specified_nm",
                    "trans.k2.torque_specified_nm",
                    "trans.clutch_oil_temp_c",
                ],
                5,
                (3, 7, 3, 1, Some(4.4)),
            ),
            p(
                "Haldex",
                &[
                    "vehicle.speed_kmh",
                    "steering.angle_deg",
                    "vehicle.yaw_dps",
                    "wheels.fl_kmh",
                    "wheels.fr_kmh",
                    "wheels.rl_kmh",
                    "wheels.rr_kmh",
                    "awd.mode",
                    "awd.estimated_torque_nm",
                    "awd.oil_pressure_bar",
                    "awd.valve_opening_pct",
                    "awd.plate_temp_c",
                ],
                5,
                (8, 4, 2, 1, Some(6.6)),
            ),
            p(
                "Thermal",
                &[
                    "vehicle.speed_kmh",
                    "ambient.outside_temp_c",
                    "engine.coolant_c",
                    "engine.oil_temp_c",
                    "engine.intake_temp_c",
                    "engine.exhaust_temp.projected_c",
                    "engine.fan_pct",
                    "trans.fluid_temp_c",
                    "trans.clutch_oil_temp_c",
                    "awd.oil_temp_c",
                    "awd.plate_temp_c",
                ],
                1,
                (5, 6, 4, 3, Some(1.0)),
            ),
            p(
                "Electrical",
                &[
                    "power.battery_v",
                    "power.battery_soc_pct",
                    "engine.generator_pct",
                    "power.generator_target_v",
                    "power.bdm_v",
                    "power.load_shed_stage",
                    "engine.rpm",
                ],
                2,
                (7, 0, 0, 0, None),
            ),
        ],
        selected: Some("Pull".into()),
        running: None,
        last: None,
    }
}
