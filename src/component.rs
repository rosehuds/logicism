use std::{rc::Rc, str::FromStr};

use druid::{
    kurbo::RoundedRect, widget::SvgData, Affine, Color, Data, Event, Insets, PaintCtx, Point, Rect,
    RenderContext, Size, Vec2, Widget,
};

use crate::{
    canvas::{Coords, BEGIN_DRAG, DESELECT_ALL},
    IDENTITY,
};

#[derive(Clone, Copy, Data, PartialEq, Eq)]
pub enum Orientation {
    North,
    East,
    South,
    West,
}

impl Orientation {
    pub fn angle(&self) -> f64 {
        match self {
            Orientation::North => 0.0,
            Orientation::East => std::f64::consts::FRAC_PI_2,
            Orientation::South => std::f64::consts::PI,
            Orientation::West => std::f64::consts::FRAC_PI_2 * 3.0,
        }
    }
}

pub struct ComponentType {
    pub size: Size,
    /// The point that is represented by the coordinates of a component when it is oriented north
    anchor_offset: Vec2,
    pub icon: SvgData,
    pub input_pins: Vec<Coords>,
    pub output_pins: Vec<Coords>,
}

impl ComponentType {
    pub fn enumerate() -> Vec<Rc<Self>> {
        let not_gate = ComponentType {
            size: Size::new(24.0, 48.0),
            anchor_offset: Vec2::new(12.0, 32.0),
            icon: SvgData::from_str(include_str!("../res/not_gate.svg")).unwrap(),
            input_pins: vec![Coords::new(0, 1)],
            output_pins: vec![Coords::new(0, -2)],
        };
        let and_gate = ComponentType {
            size: Size::new(48.0, 48.0),
            anchor_offset: Vec2::new(24.0, 32.0),
            icon: SvgData::from_str(include_str!("../res/and_gate.svg")).unwrap(),
            input_pins: vec![Coords::new(-1, 1), Coords::new(1, 1)],
            output_pins: vec![Coords::new(0, -2)],
        };
        let or_gate = ComponentType {
            size: Size::new(48.0, 48.0),
            anchor_offset: Vec2::new(24.0, 32.0),
            icon: SvgData::from_str(include_str!("../res/or_gate.svg")).unwrap(),
            input_pins: vec![Coords::new(-1, 1), Coords::new(1, 1)],
            output_pins: vec![Coords::new(0, -2)],
        };
        let nand_gate = ComponentType {
            size: Size::new(48.0, 48.0),
            anchor_offset: Vec2::new(24.0, 32.0),
            icon: SvgData::from_str(include_str!("../res/nand_gate.svg")).unwrap(),
            input_pins: vec![Coords::new(-1, 1), Coords::new(1, 1)],
            output_pins: vec![Coords::new(0, -2)],
        };
        vec![
            Rc::new(not_gate),
            Rc::new(and_gate),
            Rc::new(or_gate),
            Rc::new(nand_gate),
        ]
    }

    pub fn anchor_offset(&self, orientation: Orientation) -> Vec2 {
        let a = self.anchor_offset;
        match orientation {
            Orientation::North => a,
            Orientation::East => Vec2::new(self.size.height - a.y, a.x),
            Orientation::South => Vec2::new(self.size.width - a.x, self.size.height - a.y),
            Orientation::West => Vec2::new(a.y, self.size.width - a.x),
        }
    }

    pub fn bounding_rect(&self, coords: Coords, orientation: Orientation) -> Rect {
        let top_left = coords.to_canvas_space() - self.anchor_offset(orientation);
        let size = match orientation {
            Orientation::North | Orientation::South => self.size,
            Orientation::East | Orientation::West => Size::new(self.size.height, self.size.width),
        };
        Rect::from_origin_size(top_left, size)
    }
}

#[derive(Clone, Data)]
pub struct ComponentInstance {
    coords: Coords,
    ty: Rc<ComponentType>,
    orientation: Orientation,
}

impl ComponentInstance {
    pub fn new(coords: Coords, ty: Rc<ComponentType>, orientation: Orientation) -> Self {
        ComponentInstance {
            coords,
            ty,
            orientation,
        }
    }

    pub fn bounding_rect(&self) -> Rect {
        self.ty.bounding_rect(self.coords, self.orientation)
    }

    pub fn paint(&self, ctx: &mut PaintCtx) {
        let recenter = match self.orientation {
            Orientation::North => IDENTITY,
            Orientation::East => Affine::translate(Vec2::new(self.ty.size.height, 0.0)),
            Orientation::South => {
                Affine::translate(Vec2::new(self.ty.size.width, self.ty.size.height))
            },
            Orientation::West => Affine::translate(Vec2::new(0.0, self.ty.size.width)),
        };
        let rotate_center = recenter * Affine::rotate(self.orientation.angle());

        ctx.with_save(|ctx| {
            ctx.transform(rotate_center);
            self.ty.icon.to_piet(IDENTITY, ctx);

            ctx.transform(Affine::translate(self.anchor_offset()));
            for pin_pos in self.ty.input_pins.iter().chain(self.ty.output_pins.iter()) {
                ctx.fill(
                    Rect::from_center_size(pin_pos.to_widget_space(), Size::new(2.0, 2.0)),
                    &Color::GREEN,
                );
            }
        });
    }

    fn anchor_offset(&self) -> Vec2 {
        self.ty.anchor_offset(Orientation::North)
    }
}

#[derive(Clone, Data)]
pub struct ComponentState {
    pub instance: ComponentInstance,
    selected: bool,
    dragging: Option<Vec2>,
}

impl ComponentState {
    pub fn new(coords: Coords, ty: Rc<ComponentType>, orientation: Orientation) -> Self {
        ComponentState {
            instance: ComponentInstance::new(coords, ty, orientation),
            selected: false,
            dragging: None,
        }
    }
}

pub struct Component;

impl Widget<ComponentState> for Component {
    fn event(
        &mut self,
        ctx: &mut druid::EventCtx,
        event: &druid::Event,
        data: &mut ComponentState,
        _env: &druid::Env,
    ) {
        match event {
            Event::MouseDown(ev) => {
                if !data.selected {
                    data.selected = true;
                    ctx.request_paint();
                    if !ev.mods.ctrl() {
                        ctx.submit_command(DESELECT_ALL.with(ctx.widget_id()));
                    }
                }

                ctx.submit_command(BEGIN_DRAG.with(ev.window_pos));
                ctx.request_focus();
                ctx.set_handled();
            },
            Event::MouseUp(_) => {
                data.dragging = None;
                ctx.set_active(false);
            },
            Event::MouseMove(ev) => {
                if let Some(mouse_offset) = data.dragging {
                    data.instance.coords = Coords::from_canvas_space(ev.window_pos - mouse_offset);
                }
            },
            Event::KeyDown(ev) => {
                use druid::keyboard_types::Key;
                let mut orientation = data.instance.orientation;
                match ev.key {
                    Key::Character(ref s) if s == "w" => orientation = Orientation::North,
                    Key::Character(ref s) if s == "a" => orientation = Orientation::West,
                    Key::Character(ref s) if s == "s" => orientation = Orientation::South,
                    Key::Character(ref s) if s == "d" => orientation = Orientation::East,
                    _ => {},
                }
                if orientation != data.instance.orientation {
                    data.instance.orientation = orientation;
                    ctx.request_paint();
                }
            },
            Event::Command(c) if c.is(DESELECT_ALL) => {
                let widget_id = c.get(DESELECT_ALL).unwrap();
                if *widget_id != ctx.widget_id() {
                    data.selected = false;
                    ctx.set_active(false);
                    ctx.resign_focus();
                    ctx.request_paint();
                }
            },
            Event::Command(c) if c.is(BEGIN_DRAG) && data.selected => {
                let window_pos = c.get(BEGIN_DRAG).unwrap();
                data.dragging = Some(
                    *window_pos
                        - data.instance.anchor_offset()
                        - data.instance.bounding_rect().origin(),
                );
                ctx.set_active(true);
            },
            _ => {},
        }
    }

    fn lifecycle(
        &mut self,
        _ctx: &mut druid::LifeCycleCtx,
        _event: &druid::LifeCycle,
        _data: &ComponentState,
        _env: &druid::Env,
    ) {
    }

    fn update(
        &mut self,
        _ctx: &mut druid::UpdateCtx,
        _old_data: &ComponentState,
        _data: &ComponentState,
        _env: &druid::Env,
    ) {
    }

    fn layout(
        &mut self,
        ctx: &mut druid::LayoutCtx,
        bc: &druid::BoxConstraints,
        data: &ComponentState,
        _env: &druid::Env,
    ) -> Size {
        ctx.set_paint_insets(Insets::uniform(8.0));
        bc.constrain(data.instance.ty.size)
    }

    fn paint(&mut self, ctx: &mut druid::PaintCtx, data: &ComponentState, _env: &druid::Env) {
        data.instance.paint(ctx);
        if data.selected {
            // we're painting in widget space already so the bounding rect needs to be translated
            // back
            let selection_rect = data
                .instance
                .bounding_rect()
                .with_origin(Point::ORIGIN)
                .inflate(4.0, 4.0);
            ctx.stroke(
                RoundedRect::from_rect(selection_rect, 4.0),
                &Color::AQUA,
                1.0,
            );
        }
    }
}
