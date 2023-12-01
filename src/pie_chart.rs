use adw::subclass::prelude::*;
use gtk::prelude::*;
use gtk::{cairo, gdk, glib};

use std::f64::consts::PI;

mod imp {
    const INNER_CIRCLE_RADIUS: f64 = 0.6;
    const SPACING: f64 = 0.1;
    const MIN_WEIGHT_RATIO: f64 = 1.0 / 100.0;

    use std::cell::{Cell, RefCell};

    use gtk::pango::FontDescription;

    use super::*;

    #[derive(Debug, Default, glib::Properties)]
    #[properties(wrapper_type = super::PieChart)]
    pub struct PieChart {
        pub(super) items: RefCell<Vec<PieChartItem>>,
        highlighted_item_index: Cell<Option<usize>>,
        radius: Cell<f64>,
        width: Cell<f64>,
        height: Cell<f64>,

        #[property(get, set)]
        title: RefCell<Option<String>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PieChart {
        const NAME: &'static str = "PieChart";
        type Type = super::PieChart;
        type ParentType = gtk::DrawingArea;
    }

    #[glib::derived_properties]
    impl ObjectImpl for PieChart {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            obj.set_draw_func(glib::clone!(@weak self as widget => move |_, cr, w, h| {
                widget.draw_chart(cr, w, h);
            }));

            let motion_controller = gtk::EventControllerMotion::new();
            motion_controller.connect_motion(glib::clone!(@weak self as widget => move |_, x, y| {
                widget.highlight_item_at_point(x, y);
            }));
            obj.add_controller(motion_controller);

            let gesture_click = gtk::GestureClick::new();
            gesture_click.set_button(gdk::BUTTON_PRIMARY);
            gesture_click.connect_pressed(glib::clone!(@weak self as widget => move |_, _, x, y| {
                if let Some(item) = widget.highlight_item_at_point(x, y) {
                    println!("clicked: {}", item.title());
                }
            }));
            obj.add_controller(gesture_click);
        }
    }
    impl WidgetImpl for PieChart {}
    impl DrawingAreaImpl for PieChart {}

    impl PieChart {
        fn draw_chart(&self, context: &cairo::Context, width: i32, height: i32) {
            let width_f64: f64 = width.into();
            let height_f64: f64 = height.into();
            let radius = width_f64.min(height_f64) / 2.0;

            let xc = width_f64 / 2.0;
            let yc = height_f64 / 2.0;

            let pango_layout = self
                .obj()
                .create_pango_layout(self.title.borrow().as_deref());
            pango_layout.set_alignment(gtk::pango::Alignment::Center);
            pango_layout.set_font_description(Some(&FontDescription::from_string("Sans Thin 32")));
            let (pe, _) = pango_layout.pixel_extents();
            context.move_to(xc - (pe.width() as f64 / 2.0), yc - pe.height() as f64);
            pangocairo::functions::show_layout(context, &pango_layout);

            context.new_path();

            self.radius.set(radius);
            self.width.set(width_f64);
            self.height.set(height_f64);

            let spacing = SPACING / (2.0 * PI);

            let items = self.items.borrow_mut();
            let highlighted_item_index = self.highlighted_item_index.get();

            let total: f64 = items.iter().map(|item| item.weight()).sum();
            let new_items: Vec<_> = items
                .iter()
                .filter(|item| item.weight() / total >= MIN_WEIGHT_RATIO)
                .collect();
            let total: f64 = new_items.iter().map(|item| item.weight()).sum();

            let mut acc = 0.0;
            for (index, item) in new_items.iter().enumerate() {
                let weight = item.weight();
                let weight_ratio = weight / total;

                let highlighted = if let Some(highlighted_idx) = highlighted_item_index {
                    index == highlighted_idx
                } else {
                    false
                };

                let color = self.get_item_color(index, highlighted);
                GdkCairoContextExt::set_source_rgba(context, &color);

                let from_angle = acc + spacing;
                let to_angle = acc + weight_ratio * (2.0 * PI);
                context.arc(xc, yc, radius, from_angle, to_angle);

                item.set_start_angle(from_angle);
                item.set_end_angle(to_angle);

                let new_radius = radius * INNER_CIRCLE_RADIUS;
                let new_x = new_radius * to_angle.cos() + xc;
                let new_y = new_radius * to_angle.sin() + yc;
                context.line_to(new_x, new_y);
                context.arc_negative(xc, yc, new_radius, to_angle, from_angle);
                context.fill().expect("failed to fill");
                acc = to_angle;
            }
        }

        fn highlight_item_at_point(&self, x: f64, y: f64) -> Option<PieChartItem> {
            let obj = self.obj();
            for (index, item) in self.items.borrow().iter().enumerate() {
                if self.item_at_point(item, x, y) {
                    obj.set_tooltip_text(Some(&item.title()));
                    obj.set_has_tooltip(true);
                    obj.set_cursor(gdk::Cursor::from_name("pointer", None).as_ref());
                    self.set_highlighted_item_index(Some(index));
                    return Some(item.clone());
                }
            }
            obj.set_tooltip_text(None);
            obj.set_has_tooltip(false);
            obj.set_cursor(None);
            self.set_highlighted_item_index(None);
            None
        }

        fn set_highlighted_item_index(&self, highlighted_item_index: Option<usize>) {
            if self.highlighted_item_index.get() == highlighted_item_index {
                return;
            }

            self.highlighted_item_index.set(highlighted_item_index);
            self.obj().queue_draw();
        }

        fn get_item_color(&self, index: usize, highlighted: bool) -> gdk::RGBA {
            let color = match index % 6 {
                0 => "#e01b24",
                1 => "#ff7800",
                2 => "#f6d32d",
                3 => "#33d17a",
                4 => "#3584e4",
                _ => "#9141ac",
            };
            let mut rgba = gdk::RGBA::parse(color).unwrap();
            if highlighted {
                rgba.set_red(rgba.red() / 1.1);
                rgba.set_blue(rgba.blue() / 1.1);
                rgba.set_green(rgba.green() / 1.1);
            }
            rgba
        }

        fn item_at_point(&self, item: &PieChartItem, x: f64, y: f64) -> bool {
            let max_radius = self.radius.get();
            let xc = self.width.get() / 2.0;
            let yc = self.height.get() / 2.0;
            let from_angle: f64 = item.start_angle();
            let to_angle: f64 = item.end_angle();

            let x = x - xc;
            let y = y - yc;

            let distance = (x.powi(2) + y.powi(2)).sqrt();
            let mut angle = y.atan2(x);
            if angle < 0.0 {
                angle += 2.0 * PI;
            }

            distance <= max_radius
                && distance >= (max_radius * INNER_CIRCLE_RADIUS)
                && angle >= from_angle
                && angle <= to_angle
        }
    }
}

glib::wrapper! {
    pub struct PieChart(ObjectSubclass<imp::PieChart>)
        @extends gtk::Widget, gtk::DrawingArea;
}

impl PieChart {
    pub fn add_item(&self, item: &PieChartItem) {
        let imp = self.imp();
        {
            imp.items.borrow_mut().push(item.clone());
        }
        self.queue_draw();
    }

    pub fn clear(&self) {
        let imp = self.imp();
        {
            imp.items.borrow_mut().clear();
        }
        self.queue_draw();
    }
}

mod imp2 {
    use std::cell::{Cell, RefCell};

    use super::*;

    #[derive(Default, glib::Properties)]
    #[properties(wrapper_type = super::PieChartItem)]
    pub struct PieChartItem {
        #[property(get, set)]
        title: RefCell<String>,
        #[property(get, set)]
        weight: Cell<f64>,
        #[property(get, set)]
        start_angle: Cell<f64>,
        #[property(get, set)]
        end_angle: Cell<f64>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PieChartItem {
        const NAME: &'static str = "PieChartItem";
        type Type = super::PieChartItem;
    }

    #[glib::derived_properties]
    impl ObjectImpl for PieChartItem {}
}

glib::wrapper! {
    pub struct PieChartItem(ObjectSubclass<imp2::PieChartItem>);
}

impl PieChartItem {
    pub fn new(title: &str, weight: f64) -> Self {
        glib::Object::builder()
            .property("title", title)
            .property("weight", weight)
            .build()
    }
}
