use std::time::Instant;

use egui::{emath, Color32, Frame, Pos2, Rect, Sense, Shape, Stroke, Vec2};
use rand::Rng;
use sstree::indirect_handle::{Bounds, SsTree};

struct Select {
    center: Pos2,
    radius: f32,
}

// #[derive(Debug, PartialEq)]
// struct CenterRadius2 {
//     center: [f32; 2],
//     radius: f32,
// }

// impl CenterRadius for CenterRadius2 {
//     type K = [f32; 2];

//     fn center(&self) -> &Self::K {
//         &self.center
//     }

//     fn from_center_radius(center: Self::K, radius: f32) -> Self {
//         Self { center, radius }
//     }

//     fn radius(&self) -> f32 {
//         self.radius
//     }
// }

#[derive(Debug)]
struct Drag {
    id: u64,
    last_pos: Pos2,
}

fn pos2_to_array(pos: &Pos2) -> [f32; 2] {
    [pos.x, pos.y]
}

impl Select {
    pub fn new(center: Pos2) -> Self {
        Self {
            center,
            radius: 0.0,
        }
    }

    pub fn update(&mut self, pos: Pos2) {
        let d = pos - self.center;
        self.radius = d.length();
    }
}

impl Drag {
    fn update<const M: usize>(&mut self, pos: Pos2, tree: &mut SsTree<u64, [f32; 2], M>) {
        //todo!()
        let mut element = tree
            .remove_if(
                &Bounds {
                    center: [self.last_pos.x, self.last_pos.y],
                    radius: 1.0,
                },
                |k| *k == self.id,
            )
            .expect("failed to remove on drag");

        let d = pos - self.last_pos;
        element.center_radius.center[0] += d.x;
        element.center_radius.center[1] += d.y;
        self.last_pos = pos;
        tree.insert_entry(element);
        // let p = tree.get_by_path(&self.path).payload.clone();
        // assert!(p == payload);
    }
}

const M: usize = 8;
const LOWER_M: usize = M / 2;

#[derive(Default, PartialEq)]
enum Mode {
    #[default]
    Draw,
    Select,
    Delete,
    Drag,
}

#[derive(Default)]
struct MyEguiApp {
    shapes: Vec<Shape>,

    tree: SsTree<u64, [f32; 2], M>,

    mode: Mode,
    max_depth: usize,
    draw_points: bool,
    select_tool: Option<Select>,
    smear: bool,
    delete_radius: f32,
    insert_radius: f32,
    insert_count: u64,
    drag_tool: Option<Drag>,
}

impl eframe::App for MyEguiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let res = ui.add(egui::Slider::new(&mut self.max_depth, 0..=10));
            let res2 = ui.add(egui::Checkbox::new(&mut self.draw_points, "points"));

            ui.radio_value(&mut self.mode, Mode::Draw, "draw");
            ui.radio_value(&mut self.mode, Mode::Select, "select");
            ui.radio_value(&mut self.mode, Mode::Delete, "delete");
            ui.radio_value(&mut self.mode, Mode::Drag, "drag");

            // ui.add(egui::Checkbox::new(&mut self.select, "select"));
            // ui.add(egui::Checkbox::new(&mut self.delete, "delete"));
            // ui.add(egui::Checkbox::new(&mut self.drag, "drag"));
            ui.add(egui::Checkbox::new(&mut self.smear, "smear"));
            ui.add(egui::Slider::new(&mut self.insert_radius, 1.0..=20.0));
            ui.add(egui::Slider::new(&mut self.delete_radius, 5.0..=100.0).text("delete radius"));

            // println!("res: {:?}", res);
            Frame::dark_canvas(ui.style()).show(ui, |ui| {
                self.ui_canvas(ui, res.changed() || res2.changed());
            });
        });
    }
}

impl MyEguiApp {
    pub fn ui_canvas(&mut self, ui: &mut egui::Ui, mut changed: bool) -> egui::Response {
        // self.ui_content(ui);
        ui.heading("SS-Tree");

        let (response, painter) =
            ui.allocate_painter(ui.available_size_before_wrap(), Sense::drag());
        let to_screen = emath::RectTransform::from_to(
            Rect::from_min_size(Pos2::ZERO, response.rect.square_proportions()),
            response.rect,
        );

        match self.mode {
            Mode::Select => {
                if response.drag_started() {
                    self.select_tool = Some(Select::new(
                        response
                            .interact_pointer_pos()
                            .expect("missing pointer pos in drag start"),
                    ));
                } else if response.drag_released() {
                    self.select_tool = None;
                } else if response.dragged() {
                    if let Some(select_tool) = self.select_tool.as_mut() {
                        select_tool.update(
                            response
                                .interact_pointer_pos()
                                .expect("missing pointer pos in drag"),
                        );
                    }
                }
            }
            Mode::Delete if response.dragged() => {
                let start = Instant::now();

                let mut selected = Vec::new();

                self.tree.find_entries_within_radius(
                    &Bounds {
                        center: pos2_to_array(&response.interact_pointer_pos().unwrap()),
                        radius: self.delete_radius,
                    },
                    &mut selected,
                );
                painter.add(egui::Shape::circle_stroke(
                    response.interact_pointer_pos().unwrap(),
                    self.delete_radius,
                    Stroke::new(1.0, Color32::WHITE),
                ));

                println!("selected: {} in {:?}", selected.len(), start.elapsed());
                let start = Instant::now();
                changed = !selected.is_empty();
                let centers = selected
                    .drain(..)
                    .map(|e| e.as_ref().center)
                    .collect::<Vec<_>>();
                for point in centers {
                    self.tree.remove(&point); // FIXME: we probably want to delete by identity
                }
                println!("deleted: {:?}", start.elapsed());
            }
            Mode::Drag => {
                if response.drag_started() {
                    let mut selected = Vec::new();

                    let pos2 = response.interact_pointer_pos().unwrap();
                    self.tree.find_entries_within_radius(
                        &Bounds {
                            center: pos2_to_array(&pos2),
                            radius: 1.0,
                        },
                        &mut selected,
                    );
                    // self.tree
                    //     .paths_within_radius(&pos2_to_array(&pos2), 1.0, &mut selected);
                    println!("drag start {:?}", selected);

                    if let Some(path) = selected.pop() {
                        self.drag_tool = Some(Drag {
                            id: path.payload,
                            last_pos: pos2,
                        });
                        // self.tree.remove_by_path(&path);
                        changed = true;
                    }
                } else if response.drag_released() {
                    self.drag_tool = None;
                    changed = true;
                } else if response.dragged() {
                    if let Some(drag_tool) = self.drag_tool.as_mut() {
                        drag_tool.update(
                            response
                                .interact_pointer_pos()
                                .expect("missing pointer pos in drag"),
                            &mut self.tree,
                        );
                    }
                    changed = true;
                    println!("drag update {:?}", self.drag_tool);
                }
            }
            Mode::Draw => {
                if response.clicked() || self.smear {
                    let from_screen = to_screen.inverse();

                    if let Some(pointer_pos) = response.interact_pointer_pos() {
                        let canvas_pos = from_screen * pointer_pos;
                        println!("interact: {:?}", canvas_pos);
                        self.tree.insert(
                            self.insert_count,
                            [pointer_pos.x, pointer_pos.y],
                            self.insert_radius,
                        );
                        self.insert_count += 1;
                        changed = true;
                        // self.shapes
                        //     .push(egui::Shape::circle_stroke(pointer_pos, 10.0, self.stroke));
                        // if current_line.last() != Some(&canvas_pos) {
                        //     current_line.push(canvas_pos);
                        //     response.mark_changed();
                        // }
                        // response.mark_changed();
                    }
                }
            }
            _ => (),
        }
        if changed {
            self.shapes.clear();
            draw_tree(
                painter.clip_rect(),
                &mut self.shapes,
                &self.tree.root,
                self.max_depth,
                self.draw_points,
                &self.tree.pool,
            );
        }

        painter.extend(self.shapes.clone());
        // for i in 0..100 {
        //     tree.insert(Element::new(
        //         [rng.gen_range(200.0..600.0), rng.gen_range(200.0..600.0)],
        //         2.0,
        //         i,
        //     ));
        // }

        if let Some(select_tool) = self.select_tool.as_ref() {
            painter.add(egui::Shape::circle_stroke(
                select_tool.center,
                select_tool.radius,
                Stroke::new(1.0, Color32::WHITE),
            ));

            let mut selected = Vec::new();

            let start = Instant::now();
            if !false {
                self.tree.find_entries_within_radius(
                    &Bounds {
                        center: [select_tool.center.x, select_tool.center.y],
                        radius: select_tool.radius,
                    },
                    &mut selected,
                );
            } else {
                todo!()
                // let mut paths = Vec::new();
                // self.tree.paths_within_radius(
                //     &[select_tool.center.x, select_tool.center.y],
                //     select_tool.radius,
                //     &mut paths,
                // );

                // for path in paths {
                //     selected.push(self.tree.get_by_path(&path))
                // }
            }
            println!("selected: {} {:?}", selected.len(), start.elapsed());

            painter.extend(selected.iter().map(|p| {
                let center = Pos2::new(p.center_radius.center[0], p.center_radius.center[1]);
                egui::Shape::circle_filled(center, p.center_radius.radius + 0.5, Color32::BLUE)
            }));
        }

        response
    }
}

const COLORS: [Color32; 9] = [
    Color32::RED,
    Color32::GREEN,
    Color32::BLUE,
    Color32::LIGHT_RED,
    Color32::LIGHT_GREEN,
    Color32::LIGHT_BLUE,
    Color32::DARK_RED,
    Color32::DARK_GREEN,
    Color32::DARK_BLUE,
];

fn draw_tree<const M: usize>(
    bounds: Rect,
    shapes: &mut Vec<Shape>,
    node: &sstree::indirect_handle::InnerLink<u64, [f32; 2], M>,
    max_level: usize,
    draw_points: bool,
    pool: &sstree::indirect_handle::NodePool<u64, [f32; 2], M>,
) {
    let mut stack = Vec::new();
    stack.push((node, 0));
    let mut element_color = 0;
    while let Some((node, level)) = stack.pop() {
        if level > max_level {
            continue;
        }

        let Bounds { center, radius } = node.center_radius;

        if !overlaps(bounds, center, radius) {
            continue;
        }

        let circle = egui::Shape::circle_stroke(
            Pos2::new(center[0], center[1]),
            radius,
            Stroke::new(1.0, COLORS[level]),
        );

        shapes.push(circle);
        shapes.push(egui::Shape::circle_stroke(
            Pos2::new(center[0], center[1]),
            1.0,
            Stroke::new(1.0, COLORS[level]),
        ));

        let links = pool.get(node.links);
        match links {
            sstree::indirect_handle::Node::Inner(nodes) => {
                // level_color.inc();
                for node in nodes.iter() {
                    stack.push((node, level + 1))
                }
                // level_color.dec();
            }
            sstree::indirect_handle::Node::Leaf(points) => {
                if draw_points {
                    for point in points.iter() {
                        let point = egui::Shape::circle_filled(
                            Pos2::new(point.center_radius.center[0], point.center_radius.center[1]),
                            point.center_radius.radius,
                            COLORS[element_color % COLORS.len()],
                        );
                        shapes.push(point);
                    }
                    element_color += 1;
                }
            }
        }
    }
}

fn overlaps(bounds: Rect, centroid: [f32; 2], radius: f32) -> bool {
    // being cheap: approximate circle by square...
    let centroid: Vec2 = centroid.into();
    let r = Vec2::new(radius, radius);
    let rect = Rect {
        min: (centroid - r).to_pos2(),
        max: (centroid + r).to_pos2(),
    };
    bounds.intersects(rect)
}

fn main() {
    let mut tree = SsTree::new(LOWER_M);
    let mut rng = rand::thread_rng();

    if !false {
        for _ in 0..10 {
            println!("insert ...");
            for i in 0..100000 {
                tree.insert(
                    i,
                    [rng.gen_range(200.0..9000.0), rng.gen_range(200.0..90000.0)],
                    5.0,
                );
            }
        }
    } else {
        for i in 0..10 {
            tree.insert(
                i,
                [rng.gen_range(200.0..600.0), rng.gen_range(200.0..600.0)],
                5.0,
            );
        }
    }
    let app = MyEguiApp {
        shapes: Vec::new(),
        tree,
        mode: Mode::Draw,
        max_depth: 2,
        draw_points: true,
        select_tool: None,
        smear: false,
        insert_radius: 5.0,
        delete_radius: 20.0,
        insert_count: 0,
        drag_tool: None,
    };
    let native_options = eframe::NativeOptions::default();
    eframe::run_native("sstree test", native_options, Box::new(|_| Box::new(app)));
}
