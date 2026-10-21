use crate::models::Playbook;
use crate::theme::*;
use egui::{Color32, Pos2, Rect, Response, Sense, Stroke, Ui, Vec2};
use petgraph::algo::toposort;
use petgraph::graph::{DiGraph, NodeIndex};
use std::collections::HashMap;

pub struct TaskGraph {
    pub graph: DiGraph<String, ()>,
    #[allow(dead_code)]
    pub node_map: HashMap<String, NodeIndex>,
    pub positions: HashMap<usize, Pos2>,
    pub dragging: Option<usize>,
    pub order_changed: bool,
    pub layout_type: LayoutType,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LayoutType {
    Vertical,
    /// Круговая раскладка (зарезервировано для будущего режима отображения).
    #[allow(dead_code)]
    Circular,
}

impl TaskGraph {
    pub fn from_playbook(pb: &Playbook) -> Self {
        let mut graph = DiGraph::new();
        let mut node_map = HashMap::new();

        for t in &pb.tasks {
            let idx = graph.add_node(t.name.clone());
            node_map.insert(t.name.clone(), idx);
        }

        for t in &pb.tasks {
            if let Some(&to) = node_map.get(&t.name) {
                for dep in &t.depends_on {
                    if let Some(&from) = node_map.get(dep) {
                        graph.add_edge(from, to, ());
                    }
                }
            }
        }

        if pb.tasks.len() > 1 && graph.edge_count() == 0 {
            for i in 0..pb.tasks.len() - 1 {
                let from = node_map[&pb.tasks[i].name];
                let to = node_map[&pb.tasks[i + 1].name];
                graph.add_edge(from, to, ());
            }
        }

        Self {
            graph,
            node_map,
            positions: HashMap::new(),
            dragging: None,
            order_changed: false,
            layout_type: LayoutType::Vertical,
        }
    }

    #[allow(dead_code)]
    pub fn topo_order(&self) -> Vec<String> {
        toposort(&self.graph, None)
            .unwrap_or_default()
            .into_iter()
            .map(|i| self.graph[i].clone())
            .collect()
    }

    pub fn recalculate_order(&mut self, pb: &mut Playbook) {
        if self.positions.is_empty() || pb.tasks.is_empty() {
            return;
        }

        let mut indexed_positions: Vec<(usize, Pos2)> = self
            .positions
            .iter()
            .map(|(&idx, &pos)| (idx, pos))
            .collect();

        indexed_positions.sort_by(|a, b| {
            let y_diff = a.1.y.partial_cmp(&b.1.y).unwrap();
            if y_diff == std::cmp::Ordering::Equal {
                a.1.x.partial_cmp(&b.1.x).unwrap()
            } else {
                y_diff
            }
        });

        let mut new_tasks: Vec<crate::models::TaskDef> = Vec::new();
        for (idx, _) in indexed_positions {
            if idx < pb.tasks.len() {
                new_tasks.push(pb.tasks[idx].clone());
            }
        }

        if new_tasks.len() == pb.tasks.len() {
            let old_names: Vec<String> = pb.tasks.iter().map(|t| t.name.clone()).collect();
            let new_names: Vec<String> = new_tasks.iter().map(|t| t.name.clone()).collect();
            if old_names != new_names {
                pb.tasks = new_tasks;
                self.order_changed = true;
            }
        }
    }
}

fn draw_arrowhead(painter: &egui::Painter, from: Pos2, to: Pos2, size: f32, color: Color32) {
    let direction = to - from;
    let len = direction.length();
    if len < 0.1 {
        return;
    }
    let dir = direction / len;
    let perp = Vec2::new(-dir.y, dir.x);

    let arrow_tip = to;
    let arrow_base = to - dir * size;
    let left = arrow_base + perp * (size * 0.7);
    let right = arrow_base - perp * (size * 0.7);

    painter.add(egui::Shape::convex_polygon(
        vec![arrow_tip, left, right],
        color,
        Stroke::new(2.0_f32, Color32::WHITE),
    ));
}

pub fn draw_graph(ui: &mut Ui, graph: &mut TaskGraph, pb: &mut Playbook) -> Response {
    let available = ui.available_size();
    let (resp, painter) = ui.allocate_painter(available, Sense::click_and_drag());
    let rect = resp.rect;
    let n = graph.graph.node_count();

    if n == 0 {
        return resp;
    }

    if graph.positions.is_empty() {
        match graph.layout_type {
            LayoutType::Vertical => {
                let x_center = rect.center().x;
                let y_start = rect.top() + 40.0;
                let y_spacing = 80.0;
                for i in 0..n {
                    let pos = Pos2::new(x_center, y_start + (i as f32 * y_spacing));
                    graph.positions.insert(i, pos);
                }
            }
            LayoutType::Circular => {
                let center = rect.center();
                let radius = rect.width().min(rect.height()) * 0.35;
                for i in 0..n {
                    let a = (i as f32 / n as f32) * std::f32::consts::TAU;
                    let pos = center + Vec2::new(radius * a.cos(), radius * a.sin());
                    graph.positions.insert(i, pos);
                }
            }
        }
    }

    if let Some(dragging_idx) = graph.dragging {
        if let Some(pointer_pos) = ui.input(|i| i.pointer.interact_pos()) {
            if ui.input(|i| i.pointer.button_down(egui::PointerButton::Primary)) {
                if let Some(pos) = graph.positions.get_mut(&dragging_idx) {
                    *pos = pointer_pos;
                }
            } else {
                graph.dragging = None;
                graph.recalculate_order(pb);
            }
        }
    }

    if let Some(pointer_pos) = ui.input(|i| i.pointer.interact_pos()) {
        if resp.clicked() || resp.drag_started() {
            for (i, pos) in &graph.positions {
                let node_rect = Rect::from_center_size(*pos, Vec2::new(200.0, 40.0));
                if node_rect.contains(pointer_pos) {
                    graph.dragging = Some(*i);
                    break;
                }
            }
        }
    }

    for edge in graph.graph.edge_indices() {
        let (a, b) = graph.graph.edge_endpoints(edge).unwrap();

        // Безопасное получение позиций
        let from = match graph.positions.get(&a.index()) {
            Some(pos) => *pos,
            None => continue,
        };
        let to = match graph.positions.get(&b.index()) {
            Some(pos) => *pos,
            None => continue,
        };

        painter.line_segment(
            [from, to],
            Stroke::new(5.0_f32, Color32::from_rgb(20, 25, 35)),
        );
        painter.line_segment([from, to], Stroke::new(3.0_f32, c_blue()));

        let dir = to - from;
        let len = dir.length();
        if len > 0.1 {
            let arrow_end = to - (dir / len) * 35.0;
            draw_arrowhead(&painter, from, arrow_end, 20.0_f32, c_blue());
        }
    }

    for (i, pos) in &graph.positions {
        let label = match graph.graph.node_weight(NodeIndex::new(*i)) {
            Some(l) => l,
            None => continue,
        };

        let r = Rect::from_center_size(*pos, Vec2::new(200.0, 40.0));

        let is_dragging = graph.dragging == Some(*i);
        let fill_color = if is_dragging {
            c_orange().gamma_multiply(0.5)
        } else {
            c_purple().gamma_multiply(0.4)
        };
        let stroke_color = if is_dragging { c_orange() } else { c_purple() };

        let shadow_rect = r.translate(Vec2::new(3.0, 3.0));
        painter.rect_filled(shadow_rect, 12.0, Color32::from_rgb(10, 12, 18));
        painter.rect_filled(r, 12.0, fill_color);
        painter.rect_stroke(r, 12.0, Stroke::new(3.0_f32, stroke_color));

        // ИСПРАВЛЕНО: безопасная обрезка Unicode строк
        let max_chars = 35;
        let display_text = if label.chars().count() > max_chars {
            format!("{}...", label.chars().take(max_chars).collect::<String>())
        } else {
            label.clone()
        };

        painter.text(
            *pos,
            egui::Align2::CENTER_CENTER,
            display_text,
            egui::FontId::proportional(14.0),
            Color32::WHITE,
        );
    }

    resp
}
