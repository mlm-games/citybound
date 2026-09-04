#![allow(non_snake_case)]

use std::cell::RefCell;
use std::rc::Rc;

use repose_core::{Color, Modifier, PaddingValues, Scheduler, View};
use repose_material::material3::{Scaffold, ScaffoldConfig};
use repose_platform::RenderContext;
use repose_ui::{Box as ReposeBox, Column, Row, Spacer, Text, TextStyle, ViewExt, ZStack};

type SessionRef = Rc<RefCell<UiState>>;

#[derive(Clone)]
struct UiState {
    show_menu: bool,
    menu_tab: usize,
    show_debug: bool,
    sim_hour: u32,
    sim_min: u32,
    sim_speed_log: f32,
    ui_mode: UiMode,
    planning_mode: Option<PlanningMode>,
    selected_land_use: Option<LandUse>,
    show_building: bool,
    building_pinned: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum UiMode {
    Inspection,
    Planning,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum PlanningMode {
    Roads,
    Zoning,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum LandUse {
    Residential,
    Commercial,
    Offices,
    Industrial,
    Agricultural,
    Recreational,
    Administrative,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            show_menu: false,
            menu_tab: 0,
            show_debug: false,
            sim_hour: 9,
            sim_min: 30,
            sim_speed_log: 1.0,
            ui_mode: UiMode::Planning,
            planning_mode: Some(PlanningMode::Roads),
            selected_land_use: Some(LandUse::Residential),
            show_building: true,
            building_pinned: false,
        }
    }
}

fn grass_color() -> Color {
    Color::from_rgb(201, 224, 171)
}
fn window_bg() -> Color {
    Color::from_rgba(255, 255, 255, 230)
}
fn toolbar_bg() -> Color {
    Color::from_rgba(0, 0, 0, 221)
}
fn land_use_color(lu: LandUse) -> Color {
    match lu {
        LandUse::Residential => Color::from_rgb(234, 203, 82),
        LandUse::Commercial => Color::from_rgb(213, 94, 0),
        LandUse::Offices => Color::from_rgb(30, 30, 30),
        LandUse::Industrial => Color::from_rgb(119, 66, 95),
        LandUse::Agricultural => Color::from_rgb(136, 136, 108),
        LandUse::Recreational => Color::from_rgb(124, 192, 124),
        LandUse::Administrative => Color::from_rgb(39, 150, 221),
    }
}
fn land_use_name(lu: LandUse) -> &'static str {
    match lu {
        LandUse::Residential => "Residential",
        LandUse::Commercial => "Commercial",
        LandUse::Offices => "Offices",
        LandUse::Industrial => "Industrial",
        LandUse::Agricultural => "Agricultural",
        LandUse::Recreational => "Recreational",
        LandUse::Administrative => "Administrative",
    }
}

pub fn app(_s: &mut Scheduler, _rc: &RenderContext) -> View {
    let session: SessionRef = repose_core::remember_state_with_key("citybound_state", UiState::default);

    // Scaffold's content padding is ignored for the canvas (we want edge-to-edge),
    // but Scaffold gives us the Material indication layer for free.
    Scaffold(
        move |_pad| CityboundRoot(session.clone()),
        ScaffoldConfig {
            container_color: grass_color(),
            ..Default::default()
        },
    )
}

fn CityboundRoot(session: SessionRef) -> View {
    ZStack(Modifier::new().fill_max_size()).child((
        // canvas – would be repose-canvas with michelangelo meshes; placeholder
        CanvasPlaceholder(),
        // ui overlay – ZStack again for absolute windows + anchored toolbars
        UiOverlay(session),
    ))
}

fn CanvasPlaceholder() -> View {
    ReposeBox(
        Modifier::new()
            .fill_max_size()
            .background(grass_color()),
    )
    .child(
        Column(
            Modifier::new()
                .fill_max_size()
                .padding(32.0)
                .gap(8.0),
        )
        .child(vec![
            Text("Citybound (Repose)").size(20.0).color(Color::from_rgba(0, 0, 0, 150)),
            Text("grass #c9e0ab — Monet clearColor unchanged")
                .size(11.0)
                .color(Color::from_rgba(0, 0, 0, 110)),
        ]),
    )
}

fn UiOverlay(session: SessionRef) -> View {
    // top row (windows) + center spacer + bottom toolbar
    // We use absolute layers for windows to keep original .window positions,
    // and a bottom-anchored toolbar for tools.
    let s = session.borrow().clone();
    ZStack(Modifier::new().fill_max_size()).child((
        // top-left sim time – .sim-time 9em (144dp) at 1rem (16dp)
        ReposeBox(Modifier::new().absolute().offset(Some(16.0), Some(16.0), None, None))
            .child(SimTimeWindow(session.clone())),
        // building window – .window.building 15em x 18em, top left under sim-time for now
        // original is positioned at building 2D projection; placeholder at 16, 88
        if s.show_building {
            ReposeBox(Modifier::new().absolute().offset(Some(16.0), Some(88.0), None, None))
                .child(BuildingWindow(session.clone()))
        } else {
            ReposeBox(Modifier::new())
        },
        // debug window – floating near top, width ~320
        if s.show_debug {
            ReposeBox(Modifier::new().absolute().offset(Some(320.0), Some(16.0), None, None))
                .child(DebugWindow(session.clone()))
        } else {
            ReposeBox(Modifier::new())
        },
        // menu window – .window.menu width 40em (640dp) right side, full height minus toolbar
        if s.show_menu {
            ReposeBox(Modifier::new().absolute().offset(None, Some(16.0), Some(16.0), Some(72.0)))
                .child(MenuWindow(session.clone()))
        } else {
            ReposeBox(Modifier::new())
        },
        // bottom toolbar – .ui2dTools full-width black bar
        ReposeBox(
            Modifier::new()
                .absolute()
                .offset(None, None, Some(0.0), Some(0.0))
                .fill_max_width()
                .height(56.0),
        )
        .child(BottomToolbar(session.clone())),
        // top-right small toolbar for ui modes? original has #main-toolbar in .ui2d (top)
        // replicate as floating top bar centered
        ReposeBox(Modifier::new().absolute().offset(Some(0.0), Some(16.0), None, None).fill_max_width())
            .child(
                Row(Modifier::new().fill_max_width().padding_values(PaddingValues {
                    left: 160.0,
                    right: 16.0,
                    top: 0.0,
                    bottom: 0.0,
                }))
                .child(vec![Spacer(), TopModeToolbar(session.clone()), Spacer()]),
            ),
    ))
}
fn SimTimeWindow(session: SessionRef) -> View {
    let (h, m, speed_log) = {
        let s = session.borrow();
        (s.sim_hour, s.sim_min, s.sim_speed_log)
    };
    let time_text = format!("{:02}:{:02}", h, m);
    ReposeBox(
        Modifier::new()
            .width(144.0)
            .padding(12.0)
            .background(window_bg())
            .clip_rounded(2.0),
    )
    .child(Column(Modifier::new().gap(6.0)).child(vec![
        Row(Modifier::new().gap(4.0)).child(vec![
            Text(time_text).size(18.0).color(Color::from_rgb(0, 0, 0)),
            Text("▶").size(12.0).color(Color::from_rgb(0, 72, 255)),
        ]),
        // slider placeholder – replicate antd slider marks  || 1x 4x 32x
        ReposeBox(
            Modifier::new()
                .fill_max_width()
                .height(16.0)
                .background(Color::from_rgb(220, 220, 220))
                .clip_rounded(2.0),
        )
        .child(
            ReposeBox(
                Modifier::new()
                    .width(112.0 * (speed_log / 6.0).clamp(0.0, 1.0).max(0.12))
                    .height(16.0)
                    .background(Color::from_rgb(0, 72, 255)),
            )
            .child(Text("").size(1.0)),
        ),
        Row(Modifier::new().gap(8.0)).child(vec![
            Text("||").size(9.0).color(Color::from_rgb(120, 120, 120)),
            Text("1x").size(9.0).color(Color::from_rgb(120, 120, 120)),
            Spacer(),
            Text("4x").size(9.0).color(Color::from_rgb(120, 120, 120)),
            Spacer(),
            Text("32x").size(9.0).color(Color::from_rgb(120, 120, 120)),
        ]),
    ]))
}
fn BottomToolbar(session: SessionRef) -> View {
    let s = session.borrow().clone();
    ReposeBox(
        Modifier::new()
            .fill_max_size()
            .background(toolbar_bg())
            .padding_values(PaddingValues {
                left: 8.0,
                right: 8.0,
                top: 8.0,
                bottom: 8.0,
            }),
    )
    .child(Row(Modifier::new().fill_max_size().gap(12.0)).child(vec![
        // planning mode toolbar – mirrors #planning-toolbar (roads / zoning)
        PlanningModeToolbar(session.clone()),
        // if zoning, show land-use toolbar – #zoning-toolbar with round swatches
        if s.planning_mode == Some(PlanningMode::Zoning) {
            ZoningToolbar(session.clone())
        } else {
            ReposeBox(Modifier::new())
        },
        // undo / redo – #planning-history-toolbar
        HistoryToolbar(session.clone()),
        Spacer(),
        // implement button placeholder
        ReposeBox(
            Modifier::new()
                .padding_values(PaddingValues {
                    left: 12.0,
                    right: 12.0,
                    top: 0.0,
                    bottom: 0.0,
                })
                .height(40.0)
                .background(Color::from_rgb(0, 72, 255))
                .clip_rounded(2.0),
        )
        .child(
            Column(Modifier::new().fill_max_size().padding(8.0))
                .child(Text("Implement").size(12.0).color(Color::WHITE)),
        ),
        // project selector placeholder – width 180
        ReposeBox(
            Modifier::new()
                .width(180.0)
                .height(40.0)
                .background(Color::WHITE)
                .clip_rounded(2.0)
                .padding(8.0),
        )
        .child(Text("Project 'ABC'").size(11.0).color(Color::from_rgb(80, 80, 80))),
        // menu on far right – #menu-toolbar
        MenuToolbar(session),
    ]))
}

fn ToolbarButton(active: bool, label: &str, _icon: &str) -> View {
    let bg = if active {
        Color::WHITE
    } else {
        Color::from_rgba(187, 187, 187, 200)
    };
    let fg = if active { Color::from_rgb(0, 0, 0) } else { Color::WHITE };
    ReposeBox(
        Modifier::new()
            .width(48.0)
            .height(48.0)
            .background(bg)
            .clip_rounded(2.0)
            .padding(6.0),
    )
    .child(Text(label).size(9.0).color(fg))
}

fn PlanningModeToolbar(session: SessionRef) -> View {
    let mode = session.borrow().planning_mode;
    Row(Modifier::new().gap(2.0)).child(vec![
        Clickable(
            ToolbarButton(mode == Some(PlanningMode::Roads), "Road", "🛣"),
            {
                let s = session.clone();
                move || s.borrow_mut().planning_mode = Some(PlanningMode::Roads)
            },
        ),
        Clickable(
            ToolbarButton(mode == Some(PlanningMode::Zoning), "Zone", "▦"),
            {
                let s = session.clone();
                move || s.borrow_mut().planning_mode = Some(PlanningMode::Zoning)
            },
        ),
    ])
}

fn ZoningToolbar(session: SessionRef) -> View {
    let selected = session.borrow().selected_land_use;
    let uses = [
        LandUse::Residential,
        LandUse::Commercial,
        LandUse::Offices,
        LandUse::Industrial,
        LandUse::Agricultural,
        LandUse::Recreational,
        LandUse::Administrative,
    ];
    Row(Modifier::new().gap(8.0)).child(
        uses.iter()
            .map(|lu| {
                let is_active = Some(*lu) == selected;
                let color = land_use_color(*lu);
                let name = land_use_name(*lu);
                let short = &name[0..2];
                let s = session.clone();
                let lu_copy = *lu;
                Clickable(
                    ReposeBox(
                        Modifier::new()
                            .width(44.0)
                            .height(44.0)
                            .background(color)
                            .clip_rounded(22.0)
                            .border(if is_active { 3.0 } else { 1.0 }, Color::WHITE, 22.0),
                    )
                    .child(Text(short).size(9.0).color(Color::WHITE)),
                    move || s.borrow_mut().selected_land_use = Some(lu_copy),
                )
            })
            .collect::<Vec<_>>(),
    )
}

fn HistoryToolbar(_session: SessionRef) -> View {
    Row(Modifier::new().gap(2.0)).child(vec![
        ToolbarButton(false, "↩", "undo"),
        ToolbarButton(false, "↪", "redo"),
    ])
}

fn TopModeToolbar(session: SessionRef) -> View {
    let mode = session.borrow().ui_mode;
    Row(Modifier::new().gap(2.0)).child(vec![
        Clickable(
            ToolbarButton(mode == UiMode::Inspection, "Eye", "👁"),
            {
                let s = session.clone();
                move || s.borrow_mut().ui_mode = UiMode::Inspection
            },
        ),
        Clickable(
            ToolbarButton(mode == UiMode::Planning, "Pencil", "✎"),
            {
                let s = session.clone();
                move || s.borrow_mut().ui_mode = UiMode::Planning
            },
        ),
    ])
}

fn MenuToolbar(session: SessionRef) -> View {
    let is_open = session.borrow().show_menu;
    Clickable(
        ToolbarButton(is_open, "Menu", "☰"),
        move || session.borrow_mut().show_menu = !is_open,
    )
}
fn DebugWindow(session: SessionRef) -> View {
    ReposeBox(
        Modifier::new()
            .width(320.0)
            .background(window_bg())
            .clip_rounded(2.0)
            .padding(16.0),
    )
    .child(Column(Modifier::new().gap(10.0)).child(vec![
        Row(Modifier::new().gap(8.0)).child(vec![
            Text("Debugging").size(16.0).color(Color::from_rgb(0, 0, 0)),
            Spacer(),
            Clickable(
                Text("×").size(18.0).color(Color::from_rgb(80, 80, 80)),
                {
                    let s = session.clone();
                    move || s.borrow_mut().show_debug = false
                },
            ),
        ]),
        Text("Debug Actions").size(12.0).color(Color::from_rgb(80, 80, 80)),
        Row(Modifier::new().gap(6.0)).child(vec![
            Text("Grid 10").size(10.0).color(Color::from_rgb(50, 50, 50)),
            Text("Spawn cars").size(10.0).color(Color::from_rgb(50, 50, 50)),
        ]),
        Text("Networking").size(12.0).color(Color::from_rgb(80, 80, 80)),
        Text("turns: 0 / 0  •  queues: —")
            .size(10.0)
            .color(Color::from_rgb(100, 100, 100)),
        ReposeBox(
            Modifier::new()
                .fill_max_width()
                .height(80.0)
                .background(Color::from_rgb(51, 51, 51))
                .padding(8.0),
        )
        .child(Text("log…").size(10.0).color(Color::WHITE)),
    ]))
}
fn BuildingWindow(session: SessionRef) -> View {
    let pinned = session.borrow().building_pinned;
    ReposeBox(
        Modifier::new()
            .width(240.0)
            .height(288.0)
            .background(if pinned {
                Color::WHITE
            } else {
                Color::from_rgba(255, 255, 255, 204)
            })
            .clip_rounded(2.0)
            .padding(16.0),
    )
    .child(Column(Modifier::new().gap(8.0)).child(vec![
        Row(Modifier::new().gap(6.0)).child(vec![
            Text("Building 42").size(14.0).color(Color::from_rgb(0, 0, 0)),
            Spacer(),
            Clickable(
                Text(if pinned { "📌" } else { "📍" }).size(12.0),
                {
                    let s = session.clone();
                    move || s.borrow_mut().building_pinned = !pinned
                },
            ),
            Clickable(
                Text("×").size(16.0).color(Color::from_rgb(80, 80, 80)),
                {
                    let s = session.clone();
                    move || s.borrow_mut().show_building = false
                },
            ),
        ]),
        Text("Residential • 3 households").size(11.0).color(Color::from_rgb(80, 80, 80)),
        Text("Households").size(11.0).color(Color::from_rgb(0, 0, 0)),
        ReposeBox(
            Modifier::new()
                .fill_max_width()
                .height(140.0)
                .background(Color::from_rgb(245, 245, 245))
                .padding(8.0),
        )
        .child(Column(Modifier::new().gap(4.0)).child(vec![
            Text("• Household A — 2 adults, 1 child").size(10.0),
            Text("• Household B — 1 adult").size(10.0),
            Text("• Household C — 3 adults").size(10.0),
        ])),
        // triangle pointer mimic :after
        Row(Modifier::new().gap(4.0)).child(vec![
            Text("hover inspection — click to pin").size(9.0).color(Color::from_rgb(140, 140, 140)),
        ]),
    ]))
}
fn MenuWindow(session: SessionRef) -> View {
    let tab = session.borrow().menu_tab;
    let tabs = ["About", "Credits", "Tutorial", "Settings & Controls"];
    ReposeBox(
        Modifier::new()
            .width(640.0)
            .fill_max_height()
            .background(Color::WHITE)
            .clip_rounded(2.0)
            .padding(16.0),
    )
    .child(Column(Modifier::new().gap(12.0)).child(vec![
        Row(Modifier::new().gap(6.0)).child(vec![
            Text("Citybound").size(18.0).color(Color::from_rgb(0, 0, 0)),
            Spacer(),
            Clickable(
                Text("×").size(20.0).color(Color::from_rgb(80, 80, 80)),
                {
                    let s = session.clone();
                    move || s.borrow_mut().show_menu = false
                },
            ),
        ]),
        // tabs
        Row(Modifier::new().gap(6.0)).child(
            tabs.iter()
                .enumerate()
                .map(|(i, name)| {
                    let is_active = i == tab;
                    let s = session.clone();
                    Clickable(
                        ReposeBox(
                            Modifier::new()
                                .padding_values(PaddingValues {
                                    left: 12.0,
                                    right: 12.0,
                                    top: 8.0,
                                    bottom: 8.0,
                                })
                                .background(if is_active {
                                    Color::from_rgb(0, 72, 255)
                                } else {
                                    Color::from_rgb(240, 240, 240)
                                })
                                .clip_rounded(2.0),
                        )
                        .child(Text(*name).size(12.0).color(if is_active {
                            Color::WHITE
                        } else {
                            Color::from_rgb(50, 50, 50)
                        })),
                        move || s.borrow_mut().menu_tab = i,
                    )
                })
                .collect::<Vec<_>>(),
        ),
        // content per tab – keep original text, truncated for parity
        MenuTabContent(tab, session.clone()),
    ]))
}

fn MenuTabContent(tab: usize, session: SessionRef) -> View {
    match tab {
        0 => Column(Modifier::new().gap(8.0)).child(vec![
            Text("LIVE BUILD — not a stable release. Expect nothing to work.")
                .size(11.0)
                .color(Color::from_rgb(180, 0, 0)),
            Text("Version 0.3.0 — Repose port").size(11.0),
            ReposeBox(
                Modifier::new()
                    .padding_values(PaddingValues {
                        left: 12.0,
                        right: 12.0,
                        top: 8.0,
                        bottom: 8.0,
                    })
                    .background(Color::from_rgb(249, 104, 84))
                    .clip_rounded(4.0),
            )
            .child(Text("Become a Patron → patreon.com/citybound").size(11.0).color(Color::WHITE)),
            Text("Upcoming Release").size(13.0),
            Text("• Roads • Zoning • Implement • Undo/Redo")
                .size(11.0)
                .color(Color::from_rgb(80, 80, 80)),
        ]),
        1 => Column(Modifier::new().gap(6.0)).child(vec![
            Text("Developed by aeplay aka Anselm Eickhoff").size(11.0),
            Text("With generous support of Patrons + icons8.com").size(11.0),
            Text("Cities: Munich • St Petersburg • Reykjavík • Bangkok • Singapore • Denpasar • Kuala Lumpur • Boston")
                .size(10.0)
                .color(Color::from_rgb(80, 80, 80)),
        ]),
        2 => Column(Modifier::new().gap(6.0)).child(vec![
            Text("1) Click pencil → planning mode").size(11.0),
            Text("2) Start new project → road icon → click to add nodes, double-click to finish")
                .size(11.0),
            Text("3) Zone icon → pick land-use → draw zone, double-click finish")
                .size(11.0),
            Text("4) Implement → speed up time (top-left slider) → eye icon to inspect buildings")
                .size(11.0),
        ]),
        _ => SettingsPanel(session),
    }
}

fn SettingsPanel(_session: SessionRef) -> View {
    Column(Modifier::new().gap(8.0)).child(vec![
        Text("Settings & Controls (key bindings reload required)").size(11.0),
        SettingsRow("Pan ↑", "up"),
        SettingsRow("Pan ↓", "down"),
        SettingsRow("Pan ←/→", "left / right"),
        SettingsRow("Rotate", "alt + drag"),
        SettingsRow("Zoom", "wheel / pinch"),
        SettingsRow("Implement Plan", "ctrl+enter"),
        SettingsRow("Toggle Debug", "."),
    ])
}

fn SettingsRow(label: &str, value: &str) -> View {
    Row(Modifier::new().gap(12.0)).child(vec![
        ReposeBox(Modifier::new().width(160.0)).child(Text(label).size(11.0)),
        ReposeBox(
            Modifier::new()
                .padding_values(PaddingValues {
                    left: 8.0,
                    right: 8.0,
                    top: 4.0,
                    bottom: 4.0,
                })
                .background(Color::from_rgb(240, 240, 240))
                .clip_rounded(2.0),
        )
        .child(Text(value).size(11.0)),
    ])
}
fn Clickable(view: View, on_click: impl Fn() + 'static) -> View {
    // Repose's pointer API: Modifier::on_pointer_down captures clicks.
    // Using a Box wrapper keeps layout unchanged.
    let on_click = Rc::new(on_click);
    ReposeBox(Modifier::new().on_pointer_down(move |_ev| {
        on_click();
    }))
    .child(view)
}

#[cfg(target_arch = "wasm32")]
pub fn init_wasm() {
    console_error_panic_hook::set_once();
    if web_workers::web::has_spawn_support() {
        let _ = web_sys::console::log_1(&"wasm worker threads: available".into());
    } else {
        let _ = web_sys::console::warn_1(
            &"wasm worker threads: unavailable (need COOP/COEP)".into(),
        );
    }
}
