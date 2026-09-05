#![allow(non_snake_case)]

use std::cell::RefCell;
use std::rc::Rc;

use repose_core::{
    Color, FocusRequester, FontWeight, Modifier, PaddingValues, Rect, Scheduler, View, remember,
    remember_mutable_with_key, remember_state_with_key,
};
use repose_core::input::{Key, KeyEvent, KeyEventType};
use repose_core::locals::with_content_color;
use repose_platform::RenderContext;
use repose_ui::{Box as ReposeBox, Column, Row, Spacer, Text, TextStyle, ViewExt, ZStack};
use repose_ui::scroll::{ScrollArea, remember_scroll_state};

type SessionRef = Rc<RefCell<UiState>>;

#[derive(Clone)]
struct UiState {
    show_menu: bool,
    menu_tab: usize,
    show_debug: bool,
    debug_sec_actions: bool,
    debug_sec_net: bool,
    debug_sec_log: bool,
    debug_grid_n: i32,
    debug_grid_lanes: i32,
    debug_grid_spacing: i32,
    debug_spawn_tries: i32,
    rendering_enabled: bool,
    sim_hour: u32,
    sim_min: u32,
    sim_speed_log: f32, // 0 = paused, else speed = 2^(log-1); marks || 1x 4x 32x
    ui_mode: UiMode,
    planning_mode: Option<PlanningMode>,
    current_project: Option<String>,
    projects: Vec<String>,
    project_seq: u32,
    has_redo: bool,
    selected_land_use: Option<LandUse>,
    inspected_building: Option<String>,
    building_pinned: bool,
    hovered_window: Option<HoverWindow>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum UiMode {
    None,
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
    Industrial,
    Agricultural,
    Recreational,
    Administrative,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum HoverWindow {
    Menu,
    Debug,
    Building,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            show_menu: true,
            menu_tab: 0,
            show_debug: false,
            debug_sec_actions: false,
            debug_sec_net: false,
            debug_sec_log: false,
            debug_grid_n: 10,
            debug_grid_lanes: 2,
            debug_grid_spacing: 200,
            debug_spawn_tries: 50,
            rendering_enabled: true,
            sim_hour: 0,
            sim_min: 0,
            sim_speed_log: 1.0, // 1x, like Time.initialState speed: 1
            ui_mode: UiMode::None,
            planning_mode: None,
            current_project: None,
            projects: Vec::new(),
            project_seq: 0,
            has_redo: false,
            selected_land_use: None,
            inspected_building: None,
            building_pinned: false,
            hovered_window: None,
        }
    }
}


const PRIMARY: Color = Color(0, 72, 255, 255); // @primary-color #0048ff
const GRASS: Color = Color(201, 224, 171, 255); // body bg #c9e0ab
const TOOLBAR_BG: Color = Color(0, 0, 0, 221); // .ui2dTools #000000dd
const ICON_BG: Color = Color(68, 68, 68, 255); // #bbbbbb inverted (inactive)
const ICON_BG_ACTIVE: Color = Color(255, 255, 255, 255);
const HAIRLINE: Color = Color(217, 217, 217, 255); // antd border
const LOG_BG: Color = Color(51, 51, 51, 255); // .scrollableLog #333
const PATRON: Color = Color(249, 104, 84, 255); // .become-patron #f96854

/// Land-use swatch colors: colors.js mixes each base 90% over grass.
/// Precomputed to sRGB (see colors.js mix/toLinFloat/fromLinFloat).
fn land_use_color(lu: LandUse) -> Color {
    match lu {
        LandUse::Residential => Color(234, 207, 105, 255),
        LandUse::Commercial => Color(215, 120, 75, 255),
        LandUse::Industrial => Color(135, 103, 114, 255),
        LandUse::Agricultural => Color(149, 151, 124, 255),
        LandUse::Recreational => Color(139, 198, 136, 255),
        LandUse::Administrative => Color(87, 162, 220, 255),
    }
}

#[allow(dead_code)]
fn land_use_name(lu: LandUse) -> &'static str {
    match lu {
        LandUse::Residential => "Residential",
        LandUse::Commercial => "Commercial",
        LandUse::Industrial => "Industrial",
        LandUse::Agricultural => "Agricultural",
        LandUse::Recreational => "Recreational",
        LandUse::Administrative => "Administrative",
    }
}

/// Glyphs approximating the original icons8 PNGs (black line icons).
fn land_use_glyph(lu: LandUse) -> &'static str {
    match lu {
        LandUse::Residential => "⌂",
        LandUse::Commercial => "🏪",
        LandUse::Industrial => "🏭",
        LandUse::Agricultural => "🌾",
        LandUse::Recreational => "🌲",
        LandUse::Administrative => "🏛",
    }
}

const LAND_USES: [LandUse; 6] = [
    LandUse::Residential,
    LandUse::Commercial,
    LandUse::Industrial,
    LandUse::Agricultural,
    LandUse::Recreational,
    LandUse::Administrative,
];

fn short_project_name(id: &str) -> String {
    let head: String = id.chars().take(3).collect::<String>().to_uppercase();
    format!("Project '{head}'")
}

pub fn app(_s: &mut Scheduler, _rc: &RenderContext) -> View {
    let session: SessionRef = repose_core::remember_state_with_key("citybound_state", UiState::default);
    with_content_color(Color::BLACK, || CityboundRoot(session))
}

fn CityboundRoot(session: SessionRef) -> View {
    let focus = remember(FocusRequester::new);
    let fr_attach = (*focus).clone();
    let fr_init = (*focus).clone();
    let fr_click = (*focus).clone();
    let s_key = session.clone();
    ZStack(
        Modifier::new()
            .fill_max_size()
            .background(GRASS)
            .focusable(true)
            .focus_requester(fr_attach)
            .on_globally_positioned(move |_| {
                fr_init.request_focus();
            })
            .on_key_event(move |ke: KeyEvent| handle_shortcut(&s_key, ke))
            .on_pointer_down(move |_| {
                fr_click.request_focus();
            }),
    )
    .child((
        CanvasPlaceholder(session.clone()),
        UiOverlay(session),
    ))
}

/// Global shortcuts from the Settings tab (mirrors the original
/// Mousetrap bindings + PlanningMenu useInputBinding). Returns true
/// when the event was consumed. Pattern follows renamite's
/// `handle_viewport_key`: match on (key, command, shift, alt).
fn handle_shortcut(session: &SessionRef, event: KeyEvent) -> bool {
    if event.event_type != KeyEventType::Down || event.is_repeat {
        return false;
    }
    let m = &event.modifiers;
    match (&event.key, m.command, m.shift, m.alt) {
        (Key::Enter, true, false, false) => {
            let mut st = session.borrow_mut();
            if st.current_project.is_some() {
                st.current_project = None;
                st.planning_mode = None;
                st.selected_land_use = None;
                st.has_redo = false;
                true
            } else {
                false
            }
        }
        (Key::Character('z'), true, false, false) => {
            if session.borrow().current_project.is_some() {
                session.borrow_mut().has_redo = true;
                true
            } else {
                false
            }
        }
        (Key::Character('z'), true, true, false)
        | (Key::Character('y'), true, false, false) => {
            if session.borrow().has_redo {
                session.borrow_mut().has_redo = false;
                true
            } else {
                false
            }
        }
        (Key::Character('.'), false, false, false) => {
            let v = !session.borrow().show_debug;
            session.borrow_mut().show_debug = v;
            true
        }
        _ => false,
    }
}

/// Grass canvas with a few mock building masses so inspection has
/// something to pick (mirrors the 3D world behind the original UI).
fn CanvasPlaceholder(session: SessionRef) -> View {
    let inspection = session.borrow().ui_mode == UiMode::Inspection;
    ZStack(Modifier::new().fill_max_size()).child((
        MockBuilding(session.clone(), "bld:1", 480.0, 300.0, 96.0, 64.0, inspection),
        MockBuilding(session.clone(), "bld:2", 610.0, 350.0, 72.0, 92.0, inspection),
        MockBuilding(session.clone(), "bld:3", 400.0, 390.0, 64.0, 64.0, inspection),
    ))
}

fn MockBuilding(
    session: SessionRef,
    id: &str,
    left: f32,
    top: f32,
    w: f32,
    h: f32,
    inspection: bool,
) -> View {
    let base = Modifier::new()
        .absolute()
        .offset(Some(left), Some(top), None, None)
        .width(w)
        .height(h)
        .background(Color(238, 238, 238, 255))
        .border(1.0, Color(150, 150, 150, 255), 1.0);
    if !inspection {
        return ReposeBox(base);
    }
    let id_owned = id.to_string();
    let s_enter = session.clone();
    let s_leave = session.clone();
    let s_down = session.clone();
    let id_enter = id_owned.clone();
    let id_down = id_owned.clone();
    ReposeBox(
        base.on_pointer_enter(move |_| {
            let pinned = s_enter.borrow().building_pinned;
            if !pinned {
                s_enter.borrow_mut().inspected_building = Some(id_enter.clone());
            }
        })
        .on_pointer_leave(move |_| {
            let pinned = s_leave.borrow().building_pinned;
            if !pinned {
                s_leave.borrow_mut().inspected_building = None;
            }
        })
        .on_pointer_down(move |_| {
            s_down.borrow_mut().inspected_building = Some(id_down.clone());
            s_down.borrow_mut().building_pinned = true;
        }),
    )
}


fn UiOverlay(session: SessionRef) -> View {
    let s = session.borrow().clone();
    ZStack(Modifier::new().fill_max_size()).child((
        ReposeBox(
            Modifier::new()
                .absolute()
                .offset(None, Some(-2.0), Some(20.0), None),
        )
        .child(
            Column(Modifier::new().gap(0.0)).child(vec![
                Text("Citybound")
                    .size(24.0)
                    .color(Color(0, 0, 0, 77))
                    .letter_spacing(2.0),
                Text(format!("v{}", env!("CARGO_PKG_VERSION")))
                    .size(9.6)
                    .color(Color(0, 0, 0, 51)),
            ]),
        ),
        ReposeBox(
            Modifier::new()
                .absolute()
                .offset(Some(16.0), Some(16.0), None, None),
        )
        .child(SimTime(session.clone())),
        if s.ui_mode == UiMode::Inspection && s.inspected_building.is_some() {
            ReposeBox(
                Modifier::new()
                    .absolute()
                    .offset(Some(16.0), Some(88.0), None, None),
            )
            .child(BuildingWindow(session.clone()))
        } else {
            ReposeBox(Modifier::new())
        },
        if s.ui_mode == UiMode::Inspection && s.inspected_building.is_none() {
            ReposeBox(
                Modifier::new()
                    .absolute()
                    .offset(None, None, None, Some(80.0))
                    .fill_max_width(),
            )
            .child(
                Row(Modifier::new().fill_max_width().justify_content(
                    repose_core::JustifyContent::CENTER,
                ))
                .child(Text("Hover a building to inspect — click to pin").size(11.0).color(
                    Color(0, 0, 0, 130),
                )),
            )
        } else {
            ReposeBox(Modifier::new())
        },
        if s.show_debug {
            ReposeBox(
                Modifier::new()
                    .absolute()
                    .offset(Some(320.0), Some(16.0), None, None),
            )
            .child(DebugWindow(session.clone()))
        } else {
            ReposeBox(Modifier::new())
        },
        if s.show_menu {
            ReposeBox(
                Modifier::new()
                    .absolute()
                    .offset(None, Some(16.0), Some(16.0), Some(72.0)),
            )
            .child(MenuWindow(session.clone()))
        } else {
            ReposeBox(Modifier::new())
        },
        ReposeBox(
            Modifier::new()
                .absolute()
                .offset(None, None, Some(0.0), Some(0.0))
                .fill_max_width()
                .height(64.0),
        )
        .child(BottomToolbar(session.clone())),
    ))
}

fn SimTime(session: SessionRef) -> View {
    let (h, m) = {
        let s = session.borrow();
        (s.sim_hour, s.sim_min)
    };
    ReposeBox(Modifier::new().width(144.0).alpha(0.7)).child(
        Column(Modifier::new().gap(4.0)).child(vec![
            Text(format!("{h:02}:{m:02}")).size(17.6).color(Color::BLACK),
            SpeedSlider(session),
            Row(Modifier::new().fill_max_width().gap(0.0)).child(vec![
                Text("||").size(11.0).color(Color(90, 90, 90, 255)),
                ReposeBox(Modifier::new().width(10.0)),
                Text("1x").size(11.0).color(Color(90, 90, 90, 255)),
                Spacer(),
                Text("4x").size(11.0).color(Color(90, 90, 90, 255)),
                Spacer(),
                Text("32x").size(11.0).color(Color(90, 90, 90, 255)),
            ]),
        ]),
    )
}

/// Thin antd-like slider: 4dp rail, round thumb, marks handled above.
/// Click/drag on the 112x20 hit area sets sim_speed_log in 0..=6.
fn SpeedSlider(session: SessionRef) -> View {
    let log = session.borrow().sim_speed_log;
    let t = (log / 6.0).clamp(0.0, 1.0);
    let track_rect = remember_state_with_key("speed_track", Rect::default);
    let dragging = remember_mutable_with_key("speed_drag", || false);

    let set_from_x = Rc::new(move |x: f32, rect: Rect, session: &SessionRef| {
        if rect.w <= 1.0 {
            return;
        }
        let frac = ((x - rect.x) / rect.w).clamp(0.0, 1.0);
        session.borrow_mut().sim_speed_log = (frac * 6.0).round().clamp(0.0, 6.0);
    });

    let tr_down = track_rect.clone();
    let s_down = session.clone();
    let set_down = set_from_x.clone();
    let drag_down = dragging.clone();
    let tr_move = track_rect.clone();
    let s_move = session.clone();
    let set_move = set_from_x.clone();
    let drag_move = dragging.clone();
    let drag_up = dragging.clone();
    let drag_cancel = dragging.clone();
    let tr_pos = track_rect.clone();

    ZStack(
        Modifier::new()
            .width(112.0)
            .height(20.0)
            .on_globally_positioned(move |rect| {
                *tr_pos.borrow_mut() = rect;
            })
            .on_pointer_down(move |pe| {
                drag_down.set(true);
                let r = *tr_down.borrow();
                set_down(pe.position_in_window().x, r, &s_down);
            })
            .on_pointer_move(move |pe| {
                if drag_move.with(|v| *v) {
                    let r = *tr_move.borrow();
                    set_move(pe.position_in_window().x, r, &s_move);
                }
            })
            .on_pointer_up(move |_| {
                drag_up.set(false);
            })
            .on_pointer_cancel(move |_| {
                drag_cancel.set(false);
            }),
    )
    .child((
        ReposeBox(
            Modifier::new()
                .absolute()
                .offset(Some(0.0), Some(8.0), None, None)
                .width(112.0)
                .height(4.0)
                .background(Color(217, 217, 217, 255))
                .clip_rounded(2.0),
        ),
        ReposeBox(
            Modifier::new()
                .absolute()
                .offset(Some(0.0), Some(8.0), None, None)
                .width(112.0 * t)
                .height(4.0)
                .background(PRIMARY)
                .clip_rounded(2.0),
        ),
        ReposeBox(
            Modifier::new()
                .absolute()
                .offset(Some(t * (112.0 - 14.0)), Some(3.0), None, None)
                .width(14.0)
                .height(14.0)
                .background(Color::WHITE)
                .border(2.0, PRIMARY, 7.0)
                .clip_rounded(7.0),
        ),
    ))
}


fn BottomToolbar(session: SessionRef) -> View {
    let s = session.borrow().clone();
    let in_planning = s.ui_mode == UiMode::Planning;
    let has_project = s.current_project.is_some();

    let mut items: Vec<View> = Vec::new();
    items.push(MainModeToolbar(session.clone()));

    if in_planning {
        if has_project {
            let current = s.current_project.clone().unwrap_or_default();
            items.push(ProjectSelect(session.clone(), current));
            items.push(ImplementButton(session.clone()));
            items.push(HistoryToolbar(session.clone()));
            items.push(PlanningModeToolbar(session.clone()));
            if s.planning_mode == Some(PlanningMode::Zoning) {
                items.push(ZoningToolbar(session.clone()));
            }
        } else if s.projects.is_empty() {
            items.push(StartProjectButton(session.clone()));
        } else {
            items.push(ProjectSelect(session.clone(), String::new()));
        }
    }

    items.push(Spacer());
    items.push(MenuToolbar(session));

    ReposeBox(
        Modifier::new()
            .fill_max_size()
            .background(TOOLBAR_BG)
            .padding_values(PaddingValues {
                left: 8.0,
                right: 8.0,
                top: 8.0,
                bottom: 8.0,
            }),
    )
    .child(Row(Modifier::new().fill_max_size().gap(8.0)).child(items))
}

/// 48x48 transparent button with an inner icon tile, like .toolbar button
/// + .button-icon (inactive dark tile w/ light glyph, active white w/ dark).
fn ToolbarButton(active: bool, enabled: bool, glyph: &str) -> View {
    let (bg, fg) = if active {
        (ICON_BG_ACTIVE, Color::BLACK)
    } else {
        (ICON_BG, Color::WHITE)
    };
    let alpha = if enabled { 1.0 } else { 0.31 };
    Row(Modifier::new()
        .width(48.0)
        .height(48.0)
        .align_items(repose_core::AlignItems::CENTER)
        .justify_content(repose_core::JustifyContent::CENTER)
        .alpha(alpha))
    .child(
        ReposeBox(
            Modifier::new()
                .width(48.0)
                .height(32.0)
                .background(bg)
                .clip_rounded(2.0)
                .border(
                    1.0,
                    if active { Color::TRANSPARENT } else { Color::TRANSPARENT },
                    2.0,
                )
                .padding_values(PaddingValues {
                    left: 0.0,
                    right: 0.0,
                    top: 0.0,
                    bottom: 0.0,
                }),
        )
        .child(
            Row(Modifier::new()
                .fill_max_size()
                .align_items(repose_core::AlignItems::CENTER)
                .justify_content(repose_core::JustifyContent::CENTER))
            .child(Text(glyph).size(18.0).color(fg)),
        ),
    )
}

fn Clickable(view: View, on_click: impl Fn() + 'static) -> View {
    let on_click = Rc::new(on_click);
    ReposeBox(Modifier::new().on_pointer_down(move |_ev| {
        on_click();
    }))
    .child(view)
}

fn ClickableEnabled(view: View, enabled: bool, on_click: impl Fn() + 'static) -> View {
    if !enabled {
        return view;
    }
    Clickable(view, on_click)
}


fn MainModeToolbar(session: SessionRef) -> View {
    let mode = session.borrow().ui_mode;
    Row(Modifier::new().gap(2.0)).child(vec![
        Clickable(
            ToolbarButton(mode == UiMode::Inspection, true, "👁"),
            {
                let s = session.clone();
                move || {
                    let mut st = s.borrow_mut();
                    st.ui_mode = if st.ui_mode == UiMode::Inspection {
                        UiMode::None
                    } else {
                        UiMode::Inspection
                    };
                }
            },
        ),
        Clickable(
            ToolbarButton(mode == UiMode::Planning, true, "✎"),
            {
                let s = session.clone();
                move || {
                    let mut st = s.borrow_mut();
                    st.ui_mode = if st.ui_mode == UiMode::Planning {
                        UiMode::None
                    } else {
                        UiMode::Planning
                    };
                }
            },
        ),
    ])
}


fn StartProjectButton(session: SessionRef) -> View {
    Clickable(
        PrimaryButton("Start new project"),
        move || {
            let mut st = session.borrow_mut();
            st.project_seq += 1;
            let id = format!("prj{:03}", st.project_seq);
            st.projects.push(id.clone());
            st.current_project = Some(id);
            st.planning_mode = None;
            st.selected_land_use = None;
            st.has_redo = false;
        },
    )
}

fn ProjectSelect(session: SessionRef, current: String) -> View {
    let label = if current.is_empty() {
        "Open a project".to_string()
    } else {
        short_project_name(&current)
    };
    Clickable(
        ReposeBox(
            Modifier::new()
                .width(180.0)
                .height(32.0)
                .background(Color::WHITE)
                .border(1.0, HAIRLINE, 2.0)
                .clip_rounded(2.0)
                .padding_values(PaddingValues {
                    left: 11.0,
                    right: 11.0,
                    top: 0.0,
                    bottom: 0.0,
                }),
        )
        .child(
            Row(Modifier::new()
                .fill_max_size()
                .align_items(repose_core::AlignItems::CENTER))
            .child(vec![
                Text(label)
                    .size(14.0)
                    .color(Color(0, 0, 0, 165))
                    .single_line()
                    .overflow_ellipsize(),
                Spacer(),
                Text("▾").size(12.0).color(Color(0, 0, 0, 140)),
            ]),
        ),
        move || {
            let mut st = session.borrow_mut();
            if st.projects.is_empty() {
                return;
            }
            let idx = st
                .current_project
                .as_ref()
                .and_then(|c| st.projects.iter().position(|p| p == c))
                .map(|i| (i + 1) % st.projects.len())
                .unwrap_or(0);
            st.current_project = Some(st.projects[idx].clone());
        },
    )
}

fn PrimaryButton(label: &str) -> View {
    ReposeBox(
        Modifier::new()
            .height(32.0)
            .background(PRIMARY)
            .clip_rounded(2.0)
            .padding_values(PaddingValues {
                left: 15.0,
                right: 15.0,
                top: 0.0,
                bottom: 0.0,
            }),
    )
    .child(
        Row(Modifier::new()
            .fill_max_size()
            .align_items(repose_core::AlignItems::CENTER)
            .justify_content(repose_core::JustifyContent::CENTER))
        .child(Text(label).size(14.0).color(Color::WHITE).single_line()),
    )
}

fn ImplementButton(session: SessionRef) -> View {
    Clickable(PrimaryButton("Implement"), move || {
        let mut st = session.borrow_mut();
        st.current_project = None;
        st.planning_mode = None;
        st.selected_land_use = None;
        st.has_redo = false;
    })
}


fn HistoryToolbar(session: SessionRef) -> View {
    let (has_undo, has_redo) = {
        let s = session.borrow();
        (s.current_project.is_some(), s.has_redo)
    };
    Row(Modifier::new().gap(2.0)).child(vec![
        ClickableEnabled(
            ToolbarButton(false, has_undo, "↩"),
            has_undo,
            {
                let s = session.clone();
                move || {
                    s.borrow_mut().has_redo = true;
                }
            },
        ),
        ClickableEnabled(
            ToolbarButton(false, has_redo, "↪"),
            has_redo,
            {
                let s = session.clone();
                move || {
                    s.borrow_mut().has_redo = false;
                }
            },
        ),
    ])
}


fn PlanningModeToolbar(session: SessionRef) -> View {
    let mode = session.borrow().planning_mode;
    Row(Modifier::new().gap(2.0)).child(vec![
        Clickable(
            ToolbarButton(mode == Some(PlanningMode::Roads), true, "🛣"),
            {
                let s = session.clone();
                move || {
                    s.borrow_mut().planning_mode = Some(PlanningMode::Roads);
                }
            },
        ),
        Clickable(
            ToolbarButton(mode == Some(PlanningMode::Zoning), true, "▦"),
            {
                let s = session.clone();
                move || {
                    let mut st = s.borrow_mut();
                    st.planning_mode = Some(PlanningMode::Zoning);
                    st.selected_land_use = None;
                }
            },
        ),
    ])
}


fn ZoningToolbar(session: SessionRef) -> View {
    let selected = session.borrow().selected_land_use;
    Row(Modifier::new().gap(8.0)).child(
        LAND_USES
            .iter()
            .map(|lu| {
                let is_active = Some(*lu) == selected;
                let color = land_use_color(*lu);
                let glyph = land_use_glyph(*lu);
                let s = session.clone();
                let lu_copy = *lu;
                Clickable(
                    ReposeBox(Modifier::new().width(44.0).height(44.0)).child(
                        ReposeBox(
                            Modifier::new()
                                .width(44.0)
                                .height(44.0)
                                .background(color)
                                .clip_rounded(22.0)
                                .border(
                                    if is_active { 3.0 } else { 1.0 },
                                    if is_active {
                                        Color::WHITE
                                    } else {
                                        Color(85, 85, 85, 255)
                                    },
                                    22.0,
                                ),
                        )
                        .child(
                            Row(Modifier::new()
                                .fill_max_size()
                                .align_items(repose_core::AlignItems::CENTER)
                                .justify_content(repose_core::JustifyContent::CENTER))
                            .child(Text(glyph).size(20.0).color(Color::BLACK)),
                        ),
                    ),
                    move || s.borrow_mut().selected_land_use = Some(lu_copy),
                )
            })
            .collect::<Vec<_>>(),
    )
}

fn MenuToolbar(session: SessionRef) -> View {
    let is_open = session.borrow().show_menu;
    Clickable(
        ToolbarButton(is_open, true, "☰"),
        move || session.borrow_mut().show_menu = !is_open,
    )
}

fn WindowChrome(
    session: SessionRef,
    target: HoverWindow,
    width: f32,
    unhovered_alpha: f32,
    content: View,
) -> View {
    let hovered = session.borrow().hovered_window == Some(target);
    let s_enter = session.clone();
    let s_leave = session;
    ReposeBox(
        Modifier::new()
            .width(width)
            .background(Color::WHITE)
            .clip_rounded(2.0)
            .padding(16.0)
            .alpha(if hovered { 1.0 } else { unhovered_alpha })
            .on_pointer_enter(move |_| {
                s_enter.borrow_mut().hovered_window = Some(target);
            })
            .on_pointer_leave(move |_| {
                if s_leave.borrow().hovered_window == Some(target) {
                    s_leave.borrow_mut().hovered_window = None;
                }
            }),
    )
    .child(content)
}

fn CloseButton(session: SessionRef, target: HoverWindow) -> View {
    Clickable(
        Text("×").size(24.0).color(Color(80, 80, 80, 255)),
        move || match target {
            HoverWindow::Menu => session.borrow_mut().show_menu = false,
            HoverWindow::Debug => session.borrow_mut().show_debug = false,
            HoverWindow::Building => {
                session.borrow_mut().inspected_building = None;
                session.borrow_mut().building_pinned = false;
            }
        },
    )
}

/// .window.building 15em x 18em, opacity 0.8 (1.0 pinned), triangle pointer.
fn BuildingWindow(session: SessionRef) -> View {
    let (id, pinned, hovered) = {
        let s = session.borrow();
        (
            s.inspected_building.clone().unwrap_or_default(),
            s.building_pinned,
            s.hovered_window == Some(HoverWindow::Building),
        )
    };
    let s_enter = session.clone();
    let s_leave = session.clone();
    ZStack(Modifier::new().width(240.0).height(296.0)).child((
        ReposeBox(
            Modifier::new()
                .absolute()
                .offset(Some(0.0), Some(0.0), None, None)
                .width(240.0)
                .height(288.0)
                .background(Color::WHITE)
                .clip_rounded(2.0)
                .padding(16.0)
                .alpha(if hovered || pinned { 1.0 } else { 0.8 })
                .on_pointer_enter(move |_| {
                    s_enter.borrow_mut().hovered_window = Some(HoverWindow::Building);
                })
                .on_pointer_leave(move |_| {
                    if s_leave.borrow().hovered_window == Some(HoverWindow::Building) {
                        s_leave.borrow_mut().hovered_window = None;
                    }
                }),
        )
        .child(
            Column(Modifier::new().gap(6.0)).child(vec![
                Row(Modifier::new().gap(6.0)).child({
                    let mut r = vec![Text(format_id(&id))
                        .size(12.8)
                        .color(Color(60, 60, 60, 255))];
                    if pinned {
                        r.push(Spacer());
                        r.push(CloseButton(session.clone(), HoverWindow::Building));
                    }
                    r
                }),
                Text("Brick Apartments").size(20.8).color(Color::BLACK),
                ReposeBox(Modifier::new().fill_max_width().height(150.0)).child(
                    Column(Modifier::new().gap(8.0)).child(vec![
                        HouseholdCard("h:7a2", "2 adults, 1 child", "Idle here."),
                        HouseholdCard("h:b41", "1 adult", "On the way to work at H:09c."),
                    ]),
                ),
            ]),
        ),
        ReposeBox(
            Modifier::new()
                .absolute()
                .offset(Some(112.0), Some(280.0), None, None)
                .width(16.0)
                .height(16.0)
                .background(Color::WHITE)
                .rotate(0.785)
                .alpha(if hovered || pinned { 1.0 } else { 0.8 }),
        ),
    ))
}

fn HouseholdCard(id: &str, members: &str, state: &str) -> View {
    Column(Modifier::new().gap(2.0)).child(vec![
        Text(format_id(id)).size(16.0).color(Color::BLACK),
        Text(members).size(12.8).color(Color(80, 80, 80, 255)),
        Text(state).size(12.8).color(Color(80, 80, 80, 255)),
    ])
}

fn format_id(id: &str) -> String {
    if id.len() > 8 {
        id.chars().take(8).collect()
    } else {
        id.to_string()
    }
}

/// .window.debug with <details> sections + .scrollableLog.
fn DebugWindow(session: SessionRef) -> View {
    WindowChrome(
        session.clone(),
        HoverWindow::Debug,
        360.0,
        0.5,
        Column(Modifier::new().gap(10.0)).child(vec![
            Row(Modifier::new().gap(8.0)).child(vec![
                Text("Debugging")
                    .size(25.6)
                    .color(Color::BLACK)
                    .font_weight(FontWeight::BOLD),
                Spacer(),
                CloseButton(session.clone(), HoverWindow::Debug),
            ]),
            DetailsSection(
                session.clone(),
                "Debug Actions",
                session.borrow().debug_sec_actions,
                {
                    let s = session.clone();
                    move || {
                        let v = !s.borrow().debug_sec_actions;
                        s.borrow_mut().debug_sec_actions = v;
                    }
                },
                DebugActions(session.clone()),
            ),
            DetailsSection(
                session.clone(),
                "Networking",
                session.borrow().debug_sec_net,
                {
                    let s = session.clone();
                    move || {
                        let v = !s.borrow().debug_sec_net;
                        s.borrow_mut().debug_sec_net = v;
                    }
                },
                Column(Modifier::new().gap(6.0)).child(vec![
                    Text("browser: 0").size(12.0).color(Color(80, 80, 80, 255)),
                    ScrollableLog(vec!["queues: —".to_string()]),
                    ScrollableLog(vec!["messages: —".to_string()]),
                ]),
            ),
            DetailsSection(
                session.clone(),
                "Simulation Log",
                session.borrow().debug_sec_log,
                {
                    let s = session.clone();
                    move || {
                        let v = !s.borrow().debug_sec_log;
                        s.borrow_mut().debug_sec_log = v;
                    }
                },
                ScrollableLog(vec!["0 [setup] kay: system ready".to_string()]),
            ),
        ]),
    )
}

fn DetailsSection(
    _session: SessionRef,
    title: &str,
    open: bool,
    toggle: impl Fn() + 'static,
    content: View,
) -> View {
    let header = Clickable(
        Row(Modifier::new().gap(6.0)).child(vec![
            Text(if open { "▾" } else { "▸" })
                .size(12.0)
                .color(Color(80, 80, 80, 255)),
            Text(title).size(14.0).color(Color::BLACK),
        ]),
        toggle,
    );
    if open {
        Column(Modifier::new().gap(6.0))
            .child(vec![header, ReposeBox(Modifier::new().padding_values(PaddingValues { left: 18.0, right: 0.0, top: 0.0, bottom: 0.0 })).child(content)])
    } else {
        Column(Modifier::new()).child(header)
    }
}

fn DebugActions(session: SessionRef) -> View {
    let s = session.borrow().clone();
    let mut rows: Vec<View> = Vec::new();
    if s.current_project.is_some() {
        rows.push(
            Row(Modifier::new().gap(8.0)).child(vec![
                Text(format!("Grid size {}", s.debug_grid_n)).size(12.0),
                Text(format!("Lanes {}", s.debug_grid_lanes)).size(12.0),
                Text(format!("Spacing {}", s.debug_grid_spacing)).size(12.0),
            ]),
        );
        rows.push(SmallButton("Plan grid", {
            let _s = session.clone();
            move || {}
        }));
    } else {
        rows.push(Text("(open a project to plan a grid)").size(12.8));
    }
    rows.push(
        Row(Modifier::new().gap(8.0)).child(vec![
            Text(format!("Cars per lane (tries) {}", s.debug_spawn_tries)).size(12.0),
            SmallButton("Spawn cars", {
                let _s = session.clone();
                move || {}
            }),
        ]),
    );
    let label = if s.rendering_enabled {
        "Disable rendering"
    } else {
        "Enable rendering"
    };
    rows.push(SmallButton(label, move || {
        let v = !session.borrow().rendering_enabled;
        session.borrow_mut().rendering_enabled = v;
    }));
    Column(Modifier::new().gap(8.0)).child(rows)
}

fn SmallButton(label: &str, on_click: impl Fn() + 'static) -> View {
    Clickable(
        ReposeBox(
            Modifier::new()
                .background(Color::WHITE)
                .border(1.0, HAIRLINE, 2.0)
                .clip_rounded(2.0)
                .padding_values(PaddingValues {
                    left: 8.0,
                    right: 8.0,
                    top: 4.0,
                    bottom: 4.0,
                }),
        )
        .child(Text(label).size(12.0).color(Color(60, 60, 60, 255))),
        on_click,
    )
}

fn ScrollableLog(lines: Vec<String>) -> View {
    ReposeBox(
        Modifier::new()
            .fill_max_width()
            .height(120.0)
            .background(LOG_BG)
            .padding(16.0),
    )
    .child(
        Column(Modifier::new().gap(2.0)).child(
            lines
                .into_iter()
                .map(|l| {
                    Text(l)
                        .size(12.0)
                        .color(Color::WHITE)
                        .font_family("monospace")
                })
                .collect::<Vec<_>>(),
        ),
    )
}

/// .window.menu 40em wide, full height minus toolbar, tabs on top.
fn MenuWindow(session: SessionRef) -> View {
    let tab = session.borrow().menu_tab;
    let tabs = ["About", "Credits", "Tutorial", "Settings & Controls"];
    let s = session.clone();
    ZStack(Modifier::new().width(640.0).fill_max_height()).child((
        WindowChromeFullHeight(
            session.clone(),
            Column(Modifier::new().gap(12.0)).child(vec![
                Row(Modifier::new().gap(6.0)).child(vec![
                    Spacer(),
                    CloseButton(s, HoverWindow::Menu),
                ]),
                Row(Modifier::new().gap(6.0)).child(
                    tabs.iter()
                        .enumerate()
                        .map(|(i, name)| {
                            let sc = session.clone();
                            Clickable(
                                TabButton(i == tab, name),
                                move || sc.borrow_mut().menu_tab = i,
                            )
                        })
                        .collect::<Vec<_>>(),
                ),
                MenuScrollContent(tab),
            ]),
        ),
        ReposeBox(
            Modifier::new()
                .absolute()
                .offset(None, None, Some(8.0), Some(-8.0))
                .width(16.0)
                .height(16.0)
                .background(Color::WHITE)
                .rotate(0.785),
        ),
    ))
}

/// Menu variant of WindowChrome that stretches to full height.
fn WindowChromeFullHeight(session: SessionRef, content: View) -> View {
    let hovered = session.borrow().hovered_window == Some(HoverWindow::Menu);
    let s_enter = session.clone();
    let s_leave = session;
    ReposeBox(
        Modifier::new()
            .width(640.0)
            .fill_max_height()
            .background(Color::WHITE)
            .clip_rounded(2.0)
            .padding(16.0)
            .alpha(if hovered { 1.0 } else { 0.9 })
            .on_pointer_enter(move |_| {
                s_enter.borrow_mut().hovered_window = Some(HoverWindow::Menu);
            })
            .on_pointer_leave(move |_| {
                if s_leave.borrow().hovered_window == Some(HoverWindow::Menu) {
                    s_leave.borrow_mut().hovered_window = None;
                }
            }),
    )
    .child(content)
}

/// antd card tabs, size large: active tab white w/ primary text.
fn TabButton(active: bool, label: &str) -> View {
    ReposeBox(
        Modifier::new()
            .background(if active {
                Color::WHITE
            } else {
                Color(250, 250, 250, 255)
            })
            .border(1.0, Color(232, 232, 232, 255), 2.0)
            .clip_rounded(2.0)
            .padding_values(PaddingValues {
                left: 16.0,
                right: 16.0,
                top: 10.0,
                bottom: 10.0,
            }),
    )
    .child(
        Text(label)
            .size(16.0)
            .color(if active {
                PRIMARY
            } else {
                Color(60, 60, 60, 255)
            }),
    )
}

fn MenuTabContent(tab: usize) -> View {
    match tab {
        0 => AboutTab(),
        1 => CreditsTab(),
        2 => TutorialTab(),
        _ => SettingsTab(),
    }
}

fn MenuScrollContent(tab: usize) -> View {
    let state = remember_scroll_state("menu_tab_scroll");
    ScrollArea(
        Modifier::new().fill_max_width().weight(1.0),
        state,
        MenuTabContent(tab),
    )
}

fn PatronButton() -> View {
    ReposeBox(
        Modifier::new()
            .width(320.0)
            .height(64.0)
            .background(PATRON)
            .clip_rounded(4.0),
    )
    .child(
        Row(Modifier::new()
            .fill_max_size()
            .align_items(repose_core::AlignItems::CENTER)
            .justify_content(repose_core::JustifyContent::CENTER))
        .child(Text("Become a Patron").size(14.0).color(Color::WHITE)),
    )
}

fn AboutTab() -> View {
    Column(Modifier::new().gap(8.0)).child(vec![
        Text("Citybound").size(30.0).color(Color::BLACK).letter_spacing(2.0),
        PatronButton(),
        Text(format!("v{}", env!("CARGO_PKG_VERSION")))
            .size(22.8)
            .color(Color::BLACK)
            .font_weight(FontWeight::BOLD),
        Text("THIS IS A LIVE BUILD OF CITYBOUND AND THUS NOT A STABLE RELEASE.")
            .size(16.0)
            .color(Color::BLACK),
        Text("Expect nothing to work and a lot to be missing. See the issues below (from Github) to get an overview of the most glaring known problems and remaining tasks for the currently upcoming release.")
            .size(16.0)
            .color(Color::BLACK),
        Text("You have the newest live build.")
            .size(16.0)
            .color(Color::BLACK),
        Text("Upcoming Release:")
            .size(20.3)
            .color(Color::BLACK)
            .font_weight(FontWeight::BOLD),
        MilestoneProgress(40),
        Text("TODO:").size(18.0).color(Color::BLACK).font_weight(FontWeight::BOLD),
        Text("☐ Roads planning polish (2/5)").size(14.0),
        Text("☐ Zone implementation").size(14.0),
        Text("DONE:").size(18.0).color(Color::BLACK).font_weight(FontWeight::BOLD),
        Text("☑ Canvas rendering").size(14.0),
    ])
}

fn MilestoneProgress(percent: u32) -> View {
    Column(Modifier::new().gap(4.0)).child(vec![
        ReposeBox(
            Modifier::new()
                .fill_max_width()
                .height(8.0)
                .background(Color(245, 245, 245, 255))
                .clip_rounded(4.0),
        )
        .child(
            ReposeBox(
                Modifier::new()
                    .width(608.0 * (percent as f32 / 100.0))
                    .height(8.0)
                    .background(PRIMARY)
                    .clip_rounded(4.0),
            ),
        ),
        Text(format!("{percent}%")).size(12.0).color(Color(100, 100, 100, 255)),
    ])
}

fn CreditsTab() -> View {
    let cities = [
        "Munich",
        "Saint Petersburg",
        "Reykjavík",
        "Bangkok",
        "Singapore",
        "Denpasar",
        "Kuala Lumpur",
        "Boston",
    ];
    let mut items: Vec<View> = vec![
        Text("Citybound").size(30.0).color(Color::BLACK).letter_spacing(2.0),
        PatronButton(),
        Text("is being developed by:").size(16.0),
        Text("aeplay aka. Anselm Eickhoff").size(16.0),
        Text("With the generous support of these Patrons:")
            .size(18.0)
            .color(Color::BLACK)
            .font_weight(FontWeight::BOLD),
        Row(Modifier::new().gap(6.0)).child(vec![
            Text("Alice").size(18.0).color(PATRON),
            Text("Bob").size(14.0).color(PATRON),
            Text("Carol").size(16.0).color(PATRON),
        ]),
        Text("Icons by icons8.com").size(18.0).font_weight(FontWeight::BOLD),
        Text("Cities I developed Citybound in:")
            .size(18.0)
            .font_weight(FontWeight::BOLD),
    ];
    for city in cities {
        items.push(Text(format!("• {city}")).size(16.0));
    }
    Column(Modifier::new().gap(8.0)).child(items)
}

fn TutorialTab() -> View {
    Column(Modifier::new().gap(8.0)).child(vec![
        Text("Please note that this tutorial is super bare-bones, but it should get you going.")
            .size(16.0),
        Text("(You can open and close this whole window while following the tutorial by clicking the menu icon)")
            .size(16.0),
        Text("1) Click the pencil icon to go into planning mode.")
            .size(16.0)
            .font_weight(FontWeight::BOLD),
        Text("2) Click the \"Start a new project\" button.")
            .size(16.0)
            .font_weight(FontWeight::BOLD),
        Text("Planning Roads").size(22.8).font_weight(FontWeight::BOLD),
        Text("1) Go to road planning mode by clicking the road icon.")
            .size(16.0)
            .font_weight(FontWeight::BOLD),
        Text("2) Start a new road by clicking on the map and continue to click to add road nodes.")
            .size(16.0),
        Text("3) To finish a road, double-click when placing the last node.").size(16.0),
        Text("Planning Zones").size(22.8).font_weight(FontWeight::BOLD),
        Text("1) Go to zone planning mode by clicking the zone icon next to the road icon.")
            .size(16.0)
            .font_weight(FontWeight::BOLD),
        Text("2) Draw zone shapes by selecting a zone type, then clicking on the map to define its corners.")
            .size(16.0),
        Text("Implementing Projects").size(22.8).font_weight(FontWeight::BOLD),
        Text("Press the \"Implement\" button to implement your project plan.").size(16.0),
        Text("Further Steps").size(22.8).font_weight(FontWeight::BOLD),
        Text("Speed up time using the slider next to the clock in the top left corner and see what happens.")
            .size(16.0)
            .font_weight(FontWeight::BOLD),
        Text("Click on the eye icon and hover/click on buildings to inspect them")
            .size(16.0)
            .font_weight(FontWeight::BOLD),
    ])
}

fn SettingsTab() -> View {
    Column(Modifier::new().gap(8.0)).child(vec![
        Text("Settings & Controls").size(20.3).font_weight(FontWeight::BOLD),
        SettingsRow("Pan", "arrow keys / drag"),
        SettingsRow("Rotate", "alt + drag"),
        SettingsRow("Zoom", "wheel / pinch"),
        SettingsRow("Implement Plan", "ctrl+enter"),
        SettingsRow("Undo Plan Step", "ctrl+z"),
        SettingsRow("Redo Plan Step", "ctrl+shift+z"),
        SettingsRow("Toggle Debug", "."),
        SettingsRow("Oversampling/Retina", "2.0"),
    ])
}

fn SettingsRow(label: &str, value: &str) -> View {
    Row(Modifier::new().gap(12.0)).child(vec![
        ReposeBox(Modifier::new().width(160.0)).child(Text(label).size(14.0)),
        ReposeBox(
            Modifier::new()
                .background(Color(240, 240, 240, 255))
                .clip_rounded(2.0)
                .padding_values(PaddingValues {
                    left: 8.0,
                    right: 8.0,
                    top: 4.0,
                    bottom: 4.0,
                }),
        )
        .child(Text(value).size(14.0)),
    ])
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

#[cfg(test)]
mod shortcut_tests {
    use super::*;
    use repose_core::input::Modifiers;

    fn key_down(key: Key, command: bool, shift: bool) -> KeyEvent {
        KeyEvent {
            key,
            modifiers: Modifiers {
                command,
                shift,
                ctrl: command,
                alt: false,
                meta: false,
            },
            is_repeat: false,
            event_type: KeyEventType::Down,
            utf16_code_point: 0,
        }
    }

    fn session_with_project() -> SessionRef {
        Rc::new(RefCell::new(UiState {
            current_project: Some("prj001".to_string()),
            planning_mode: Some(PlanningMode::Roads),
            selected_land_use: Some(LandUse::Residential),
            ..UiState::default()
        }))
    }

    #[test]
    fn implement_shortcut_clears_project() {
        let s = session_with_project();
        assert!(handle_shortcut(&s, key_down(Key::Enter, true, false)));
        let st = s.borrow();
        assert!(st.current_project.is_none());
        assert!(st.planning_mode.is_none());
        assert!(st.selected_land_use.is_none());
    }

    #[test]
    fn undo_redo_shortcuts_flip_redo_flag() {
        let s = session_with_project();
        assert!(handle_shortcut(&s, key_down(Key::Character('z'), true, false)));
        assert!(s.borrow().has_redo);
        assert!(handle_shortcut(&s, key_down(Key::Character('z'), true, true)));
        assert!(!s.borrow().has_redo);
    }

    #[test]
    fn debug_toggle_shortcut_flips_window() {
        let s: SessionRef = Rc::new(RefCell::new(UiState::default()));
        let dot = KeyEvent {
            key: Key::Character('.'),
            modifiers: Modifiers::default(),
            is_repeat: false,
            event_type: KeyEventType::Down,
            utf16_code_point: 0,
        };
        assert!(!s.borrow().show_debug);
        assert!(handle_shortcut(&s, dot.clone()));
        assert!(s.borrow().show_debug);
        assert!(handle_shortcut(&s, dot));
        assert!(!s.borrow().show_debug);
    }

    #[test]
    fn shortcuts_ignore_repeats_releases_and_plain_keys() {
        let s = session_with_project();
        let mut ev = key_down(Key::Enter, true, false);
        ev.is_repeat = true;
        assert!(!handle_shortcut(&s, ev));
        let mut ev = key_down(Key::Enter, true, false);
        ev.event_type = KeyEventType::Up;
        assert!(!handle_shortcut(&s, ev));
        assert!(!handle_shortcut(&s, key_down(Key::Enter, false, false)));
        assert!(s.borrow().current_project.is_some());
    }
}
