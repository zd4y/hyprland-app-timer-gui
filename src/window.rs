/* window.rs
 *
 * Copyright 2023 zd4y
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use adw::subclass::prelude::*;
use gtk::prelude::*;
use gtk::{gio, glib};

mod imp {
    use std::{cell::RefCell, sync::Arc, time::Duration};

    use chrono::{Days, Local, NaiveDate, TimeZone, Utc};
    use gtk::glib::{Receiver, Sender};
    use hyprland_app_timer::{AppUsage, Client, SqliteDB};
    use tokio::runtime::Runtime;

    use crate::pie_chart::{PieChart, PieChartItem};

    use super::*;

    #[derive(Debug, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/zd4y/HyprlandAppTimer/ui/window.ui")]
    pub struct HyprlandAppTimerGuiWindow {
        // Template widgets
        #[template_child]
        pub calendar_date_start: TemplateChild<gtk::Calendar>,
        #[template_child]
        pub calendar_date_end: TemplateChild<gtk::Calendar>,
        #[template_child]
        pub date_range_checkbox: TemplateChild<gtk::CheckButton>,
        #[template_child]
        pub listbox: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub pie_chart: TemplateChild<PieChart>,

        sender: Sender<Message>,
        receiver: RefCell<Option<Receiver<Message>>>,
        rt: Runtime,
        db: Arc<SqliteDB>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for HyprlandAppTimerGuiWindow {
        const NAME: &'static str = "HyprlandAppTimerGuiWindow";
        type Type = super::HyprlandAppTimerGuiWindow;
        type ParentType = adw::ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[gtk::template_callbacks]
    impl HyprlandAppTimerGuiWindow {
        #[template_callback]
        fn on_date_range_checkbox_toggled(&self, checkbox: &gtk::CheckButton) {
            self.calendar_date_end.set_visible(checkbox.is_active());
            self.on_date_change();
        }

        #[template_callback]
        fn on_date_change(&self) {
            let date_start = date_glib_to_chrono(&self.calendar_date_start.date());

            let date_end = if self.date_range_checkbox.is_active() {
                let date_end = self
                    .calendar_date_end
                    .date()
                    .add_days(1)
                    .expect("failed to add days");
                date_glib_to_chrono(&date_end)
            } else {
                date_start
                    .checked_add_days(Days::new(1))
                    .expect("failed to add days")
            };

            let db = self.db.clone();
            let sender = self.sender.clone();
            self.rt.spawn(async move {
                let apps_usage = db
                    .get_apps_usage(date_start, date_end)
                    .await
                    .expect("failed to get apps usage");
                sender
                    .send(Message::AppsUsage(apps_usage))
                    .expect("failed to send apps usage");
            });
        }
    }

    impl ObjectImpl for HyprlandAppTimerGuiWindow {
        fn constructed(&self) {
            self.parent_constructed();

            let initial_datetime = self.calendar_date_start.date();

            let db = self.db.clone();
            let sender = self.sender.clone();

            self.rt.spawn(async move {
                if let Err(err) = Client::new()
                    .await
                    .expect("failed to get client")
                    .save()
                    .await
                {
                    eprintln!("Error: failed to send save message: {err}")
                }

                let date_start = date_glib_to_chrono(&initial_datetime);
                let date_end = date_start.checked_add_days(Days::new(1)).unwrap();

                let apps_usage = db
                    .get_apps_usage(date_start, date_end)
                    .await
                    .expect("failed to get apps usage");

                sender
                    .send(Message::AppsUsage(apps_usage))
                    .expect("failed to send apps usage");
            });

            self.receiver.take().unwrap().attach(None, glib::clone!(@weak self as this => @default-return glib::ControlFlow::Continue, move |msg| {
                this.handle_message(msg);
                glib::ControlFlow::Continue
            }));
        }
    }

    impl HyprlandAppTimerGuiWindow {
        fn handle_message(&self, msg: Message) {
            match msg {
                Message::AppsUsage(apps_usage) => {
                    self.pie_chart.clear();
                    while let Some(child) = self.listbox.last_child() {
                        self.listbox.remove(&child);
                    }

                    let mut total = 0.0;

                    for app_usage in apps_usage {
                        // add apps to listbox

                        let row = gtk::ListBoxRow::new();
                        let container = gtk::Box::new(gtk::Orientation::Horizontal, 20);
                        let title = gtk::Label::new(Some(&app_usage.app));
                        title.add_css_class("heading");
                        let duration = Duration::from_secs(app_usage.duration.as_secs());
                        let duration = gtk::Label::new(Some(
                            &humantime::format_duration(duration).to_string(),
                        ));
                        duration.set_halign(gtk::Align::End);
                        duration.set_hexpand(true);
                        container.append(&title);
                        container.append(&duration);
                        row.set_child(Some(&container));
                        self.listbox.append(&row);

                        let seconds = app_usage.duration.as_secs_f64();

                        // add apps to pie chart
                        self.pie_chart
                            .add_item(&PieChartItem::new(&app_usage.app, seconds));

                        total += seconds;
                    }

                    self.pie_chart.set_title(
                        humantime::format_duration(Duration::from_secs_f64(total.round()))
                            .to_string(),
                    );
                }
            }
        }
    }

    impl WidgetImpl for HyprlandAppTimerGuiWindow {}
    impl WindowImpl for HyprlandAppTimerGuiWindow {}
    impl ApplicationWindowImpl for HyprlandAppTimerGuiWindow {}
    impl AdwApplicationWindowImpl for HyprlandAppTimerGuiWindow {}

    impl Default for HyprlandAppTimerGuiWindow {
        fn default() -> Self {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(1)
                .enable_all()
                .build()
                .expect("failed to buid tokio runtime");
            let db = rt.block_on(SqliteDB::new()).expect("failed to get client");
            let (sender, receiver) = glib::MainContext::channel(glib::Priority::DEFAULT);
            HyprlandAppTimerGuiWindow {
                calendar_date_start: Default::default(),
                calendar_date_end: Default::default(),
                listbox: Default::default(),
                date_range_checkbox: Default::default(),
                sender,
                receiver: RefCell::new(Some(receiver)),
                rt,
                db: Arc::new(db),
                pie_chart: Default::default(),
            }
        }
    }

    #[derive(Debug)]
    enum Message {
        AppsUsage(Vec<AppUsage>),
    }

    fn date_glib_to_chrono(date: &glib::DateTime) -> chrono::DateTime<Utc> {
        let date =
            NaiveDate::from_ymd_opt(date.year(), date.month() as u32, date.day_of_month() as u32)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap();

        Local
            .from_local_datetime(&date)
            .unwrap()
            .with_timezone(&Utc)
    }
}

glib::wrapper! {
    pub struct HyprlandAppTimerGuiWindow(ObjectSubclass<imp::HyprlandAppTimerGuiWindow>)
        @extends gtk::Widget, gtk::Window, gtk::ApplicationWindow, adw::ApplicationWindow,
          @implements gio::ActionGroup, gio::ActionMap;
}

impl HyprlandAppTimerGuiWindow {
    pub fn new<P: glib::IsA<gtk::Application>>(application: &P) -> Self {
        glib::Object::builder()
            .property("application", application)
            .build()
    }
}
