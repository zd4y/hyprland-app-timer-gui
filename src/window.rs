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
    use std::{cell::RefCell, time::Duration};

    use chrono::{Days, Local, NaiveDate, TimeZone, Utc};
    use hyprland_app_timer::{server::Server, AppUsage, Client};
    use tokio::runtime::Runtime;

    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/zd4y/HyprlandAppTimer/ui/window.ui")]
    pub struct HyprlandAppTimerGuiWindow {
        // Template widgets
        #[template_child]
        pub header_bar: TemplateChild<adw::HeaderBar>,
        #[template_child]
        pub calendar_date_start: TemplateChild<gtk::Calendar>,
        #[template_child]
        pub calendar_date_end: TemplateChild<gtk::Calendar>,
        #[template_child]
        pub listbox: TemplateChild<gtk::ListBox>,

        rt: RefCell<Option<Runtime>>,
        client: RefCell<Option<Client>>,
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
        }
    }

    impl ObjectImpl for HyprlandAppTimerGuiWindow {
        fn constructed(&self) {
            self.parent_constructed();

            let rt = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(1)
                .enable_all()
                .build()
                .expect("failed to buid tokio runtime");

            let (sender, receiver) = glib::MainContext::channel(glib::Priority::DEFAULT);

            let initial_datetime = self.calendar_date_start.date();

            rt.spawn(async move {
                Server::save().await.expect("failed to send save signal");

                let client = Client::new().await.expect("failed to get client");

                let initial_datetime = NaiveDate::from_ymd_opt(
                    initial_datetime.year(),
                    initial_datetime.month() as u32,
                    initial_datetime.day_of_month() as u32,
                )
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap();

                let date_start = Local
                    .from_local_datetime(&initial_datetime)
                    .unwrap()
                    .with_timezone(&Utc);
                let date_end = date_start.checked_add_days(Days::new(1)).unwrap();

                let apps_usage = client
                    .get_apps_usage(date_start, date_end)
                    .await
                    .expect("failed to get apps usage");

                sender
                    .send(Message::Client(client))
                    .expect("failed to send client");

                sender
                    .send(Message::AppsUsage(apps_usage))
                    .expect("failed to send apps usage");
            });

            receiver.attach(None, glib::clone!(@weak self as this => @default-return glib::ControlFlow::Continue, move |msg| {
                this.handle_message(msg);
                glib::ControlFlow::Continue
            }));

            self.rt.replace(Some(rt));
        }
    }

    impl HyprlandAppTimerGuiWindow {
        fn handle_message(&self, msg: Message) {
            match msg {
                Message::Client(client) => {
                    self.client.replace(Some(client));
                }

                Message::AppsUsage(apps_usage) => {
                    while let Some(child) = self.listbox.last_child() {
                        self.listbox.remove(&child);
                    }

                    for app_usage in apps_usage {
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
                    }
                }
            }
        }
    }

    impl WidgetImpl for HyprlandAppTimerGuiWindow {}
    impl WindowImpl for HyprlandAppTimerGuiWindow {}
    impl ApplicationWindowImpl for HyprlandAppTimerGuiWindow {}
    impl AdwApplicationWindowImpl for HyprlandAppTimerGuiWindow {}

    #[derive(Debug)]
    enum Message {
        Client(Client),
        AppsUsage(Vec<AppUsage>),
    }
}

glib::wrapper! {
    pub struct HyprlandAppTimerGuiWindow(ObjectSubclass<imp::HyprlandAppTimerGuiWindow>)
        @extends gtk::Widget, gtk::Window, gtk::ApplicationWindow, adw::ApplicationWindow,        @implements gio::ActionGroup, gio::ActionMap;
}

impl HyprlandAppTimerGuiWindow {
    pub fn new<P: glib::IsA<gtk::Application>>(application: &P) -> Self {
        glib::Object::builder()
            .property("application", application)
            .build()
    }
}
