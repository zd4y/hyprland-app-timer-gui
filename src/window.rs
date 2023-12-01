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
    use std::cell::RefCell;

    use hyprland_app_timer::{server::Server, Client};
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

            rt.spawn(async move {
                Server::save().await.expect("failed to send save signal");
                let client = Client::new().await.expect("failed to get client");
                sender
                    .send(Message::Client(client))
                    .expect("failed to send client");
            });

            receiver.attach(None, glib::clone!(@weak self as this => @default-return glib::ControlFlow::Continue, move |msg| {
                match msg {
                    Message::Client(client) => {
                        this.client.replace(Some(client));
                    }
                }
                glib::ControlFlow::Continue
            }));

            self.rt.replace(Some(rt));
        }
    }
    impl WidgetImpl for HyprlandAppTimerGuiWindow {}
    impl WindowImpl for HyprlandAppTimerGuiWindow {}
    impl ApplicationWindowImpl for HyprlandAppTimerGuiWindow {}
    impl AdwApplicationWindowImpl for HyprlandAppTimerGuiWindow {}

    enum Message {
        Client(Client),
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
