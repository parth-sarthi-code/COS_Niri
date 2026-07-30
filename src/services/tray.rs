use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use gtk4::gio;
use gtk4::glib::{self, Variant};
use gtk4::prelude::*;

const WATCHER_NAME: &str = "org.kde.StatusNotifierWatcher";
const WATCHER_PATH: &str = "/StatusNotifierWatcher";
const ITEM_INTERFACE: &str = "org.kde.StatusNotifierItem";

const WATCHER_XML: &str = r#"
<node>
  <interface name="org.kde.StatusNotifierWatcher">
    <method name="RegisterStatusNotifierItem">
      <arg direction="in" name="service" type="s"/>
    </method>
    <method name="RegisterStatusNotifierHost">
      <arg direction="in" name="service" type="s"/>
    </method>
    <property name="RegisteredStatusNotifierItems" type="as" access="read"/>
    <property name="IsStatusNotifierHostRegistered" type="b" access="read"/>
    <property name="ProtocolVersion" type="i" access="read"/>
    <signal name="StatusNotifierItemRegistered">
      <arg name="service" type="s"/>
    </signal>
    <signal name="StatusNotifierItemUnregistered">
      <arg name="service" type="s"/>
    </signal>
    <signal name="StatusNotifierHostRegistered"/>
  </interface>
</node>
"#;

#[derive(Debug, Clone)]
pub struct TrayPixmap {
    pub width: i32,
    pub height: i32,
    pub buffer: glib::Bytes,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct TrayMenuEntry {
    pub menu_id: i32,
    pub label: String,
    pub enabled: bool,
    pub is_separator: bool,
    pub children: Vec<TrayMenuEntry>,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct TrayItem {
    pub identifier: String,
    pub bus_name: String,
    pub object_path: String,
    pub title: String,
    pub status: String,
    pub icon_name: Option<String>,
    pub pixmap: Option<TrayPixmap>,
    pub menu_path: Option<String>,
}

pub struct TrayService {
    items: RefCell<HashMap<String, TrayItem>>,
    bus: RefCell<Option<gio::DBusConnection>>,
    watcher_registration_id: RefCell<Option<gio::RegistrationId>>,
    registered_items: RefCell<HashSet<String>>,
    proxies: RefCell<HashMap<String, gio::DBusProxy>>,
    menu_proxies: RefCell<HashMap<String, gio::DBusProxy>>,
    listeners: RefCell<Vec<Rc<dyn Fn() + 'static>>>,
}

impl TrayService {
    fn new() -> Rc<Self> {
        let service = Rc::new(Self {
            items: RefCell::new(HashMap::new()),
            bus: RefCell::new(None),
            watcher_registration_id: RefCell::new(None),
            registered_items: RefCell::new(HashSet::new()),
            proxies: RefCell::new(HashMap::new()),
            menu_proxies: RefCell::new(HashMap::new()),
            listeners: RefCell::new(Vec::new()),
        });
        Self::init_dbus(&service);
        service
    }

    pub fn global() -> Rc<Self> {
        thread_local! {
            static INSTANCE: Rc<TrayService> = TrayService::new();
        }
        INSTANCE.with(|s| s.clone())
    }

    pub fn connect_change<F: Fn() + 'static>(&self, callback: F) {
        self.listeners.borrow_mut().push(Rc::new(callback));
    }

    fn notify_listeners(&self) {
        for listener in self.listeners.borrow().iter() {
            listener();
        }
    }

    pub fn get_items(&self) -> Vec<TrayItem> {
        let mut list: Vec<TrayItem> = self.items.borrow().values().cloned().collect();
        list.sort_by(|a, b| a.identifier.cmp(&b.identifier));
        list
    }

    pub fn activate(&self, identifier: &str, x: i32, y: i32) {
        if let Some(proxy) = self.proxies.borrow().get(identifier).cloned() {
            proxy.call(
                "Activate",
                Some(&(x, y).to_variant()),
                gio::DBusCallFlags::NONE,
                5000,
                None::<&gio::Cancellable>,
                |_| {}
            );
        }
    }

    #[allow(dead_code)]
    pub fn context_menu(&self, identifier: &str, x: i32, y: i32) {
        if let Some(proxy) = self.proxies.borrow().get(identifier).cloned() {
            proxy.call(
                "ContextMenu",
                Some(&(x, y).to_variant()),
                gio::DBusCallFlags::NONE,
                5000,
                None::<&gio::Cancellable>,
                |_| {}
            );
        }
    }

    fn init_dbus(this: &Rc<Self>) {
        let this_weak = Rc::downgrade(this);
        gio::bus_get(
            gio::BusType::Session,
            None::<&gio::Cancellable>,
            move |result| {
                let this = match this_weak.upgrade() {
                    Some(t) => t,
                    None => return,
                };
                let connection = match result {
                    Ok(c) => c,
                    Err(e) => {
                        eprintln!("[tray] Failed to connect to Session Bus: {}", e);
                        return;
                    }
                };
                *this.bus.borrow_mut() = Some(connection.clone());
                this.export_watcher_interface(&connection);
                this.acquire_watcher_name(&connection);
            }
        );
    }

    fn acquire_watcher_name(&self, connection: &gio::DBusConnection) {
        let this_weak = Rc::downgrade(&Self::global());
        gio::bus_own_name_on_connection(
            connection,
            WATCHER_NAME,
            gio::BusNameOwnerFlags::NONE,
            move |_conn, _name| {
                if let Some(_this) = this_weak.upgrade() {
                    println!("[tray] Owned StatusNotifierWatcher bus name");
                }
            },
            move |_conn, _name| {
                eprintln!("[tray] Lost StatusNotifierWatcher bus name");
            }
        );
    }

    fn export_watcher_interface(&self, connection: &gio::DBusConnection) {
        let node_info = match gio::DBusNodeInfo::for_xml(WATCHER_XML) {
            Ok(n) => n,
            Err(e) => {
                eprintln!("[tray] Failed to parse watcher XML: {}", e);
                return;
            }
        };
        let interface_info = match node_info.lookup_interface(WATCHER_NAME) {
            Some(i) => i,
            None => return,
        };

        let registration = connection
            .register_object(WATCHER_PATH, &interface_info)
            .method_call(|_connection, sender, _object_path, _interface_name, method_name, parameters, invocation| {
                let service = Self::global();
                match method_name {
                    "RegisterStatusNotifierItem" => {
                        if let Some(item) = parameters.child_value(0).str() {
                            let sender = sender.unwrap_or_default().to_string();
                            service.register_item(sender, item.to_string());
                        }
                        invocation.return_value(None);
                    }
                    "RegisterStatusNotifierHost" => {
                        invocation.return_value(None);
                    }
                    _ => {
                        invocation.return_error(
                            gio::IOErrorEnum::InvalidArgument,
                            &format!("Unknown method: {}", method_name)
                        );
                    }
                }
            })
            .property(|_connection, _sender, _object_path, _interface_name, property_name| {
                let service = Self::global();
                match property_name {
                    "RegisteredStatusNotifierItems" => {
                        let items: Vec<String> = service.registered_items.borrow().iter().cloned().collect();
                        items.to_variant()
                    }
                    "IsStatusNotifierHostRegistered" => true.to_variant(),
                    "ProtocolVersion" => 1i32.to_variant(),
                    _ => "".to_variant(),
                }
            })
            .build();

        if let Ok(reg_id) = registration {
            *self.watcher_registration_id.borrow_mut() = Some(reg_id);
        }
    }

    fn register_item(&self, sender: String, service: String) {
        let (bus_name, object_path) = if service.starts_with('/') {
            (sender.clone(), service)
        } else if let Some(slash_idx) = service.find('/') {
            (service[..slash_idx].to_string(), service[slash_idx..].to_string())
        } else {
            (service, "/StatusNotifierItem".to_string())
        };

        let identifier = format!("{}{}", bus_name, object_path);
        if self.registered_items.borrow().contains(&identifier) {
            return;
        }

        self.registered_items.borrow_mut().insert(identifier.clone());

        let Some(ref connection) = *self.bus.borrow() else {
            return;
        };

        let this_weak = Rc::downgrade(&Self::global());
        let id_clone = identifier.clone();
        let bus_clone = bus_name.clone();
        let path_clone = object_path.clone();

        gio::DBusProxy::new(
            connection,
            gio::DBusProxyFlags::GET_INVALIDATED_PROPERTIES,
            None,
            Some(&bus_name),
            &object_path,
            ITEM_INTERFACE,
            None::<&gio::Cancellable>,
            move |result| {
                let this = match this_weak.upgrade() {
                    Some(t) => t,
                    None => return,
                };
                match result {
                    Ok(proxy) => {
                        this.setup_item_proxy(id_clone, bus_clone, path_clone, proxy);
                    }
                    Err(e) => {
                        eprintln!("[tray] Failed to create proxy for {}: {}", id_clone, e);
                    }
                }
            }
        );
    }

    fn setup_item_proxy(&self, identifier: String, _bus_name: String, _object_path: String, proxy: gio::DBusProxy) {
        self.proxies.borrow_mut().insert(identifier.clone(), proxy.clone());

        let this_weak = Rc::downgrade(&Self::global());
        let id_clone = identifier.clone();
        proxy.connect_local("g-properties-changed", false, move |_| {
            if let Some(this) = this_weak.upgrade() {
                this.refresh_item(&id_clone);
            }
            None
        });

        let this_weak = Rc::downgrade(&Self::global());
        let id_clone2 = identifier.clone();
        let proxy_clone = proxy.clone();
        proxy.connect_local("notify::g-name-owner", false, move |_| {
            if let Some(this) = this_weak.upgrade() {
                if proxy_clone.name_owner().is_none() {
                    this.remove_item(&id_clone2);
                }
            }
            None
        });

        let this_weak = Rc::downgrade(&Self::global());
        let id_clone3 = identifier.clone();
        proxy.connect_local("g-signal", false, move |_| {
            if let Some(this) = this_weak.upgrade() {
                this.refresh_item(&id_clone3);
            }
            None
        });

        self.refresh_item(&identifier);
    }

    fn remove_item(&self, identifier: &str) {
        self.registered_items.borrow_mut().remove(identifier);
        self.items.borrow_mut().remove(identifier);
        self.proxies.borrow_mut().remove(identifier);
        self.menu_proxies.borrow_mut().remove(identifier);
        self.notify_listeners();
    }

    fn refresh_item(&self, identifier: &str) {
        let Some(proxy) = self.proxies.borrow().get(identifier).cloned() else {
            return;
        };

        let this_weak = Rc::downgrade(&Self::global());
        let id_clone = identifier.to_string();

        proxy.call(
            "org.freedesktop.DBus.Properties.GetAll",
            Some(&(ITEM_INTERFACE,).to_variant()),
            gio::DBusCallFlags::NONE,
            5000,
            None::<&gio::Cancellable>,
            move |result| {
                let this = match this_weak.upgrade() {
                    Some(t) => t,
                    None => return,
                };
                if let Ok(variant) = result {
                    this.parse_and_update_item(&id_clone, &variant);
                }
            }
        );
    }

    fn parse_properties_result(&self, result: &Variant) -> Option<HashMap<String, Variant>> {
        let inner = result.child_value(0);
        let mut map = HashMap::new();

        for i in 0..inner.n_children() {
            let entry = inner.child_value(i);
            if entry.n_children() >= 2 {
                if let Some(key) = entry.child_value(0).str() {
                    let value = entry.child_value(1);
                    let actual_value = if value.type_().is_variant() {
                        value.child_value(0)
                    } else {
                        value
                    };
                    map.insert(key.to_string(), actual_value);
                }
            }
        }

        if map.is_empty() { None } else { Some(map) }
    }

    fn parse_and_update_item(&self, identifier: &str, result: &Variant) {
        let Some(proxy) = self.proxies.borrow().get(identifier).cloned() else {
            return;
        };
        let map = self.parse_properties_result(result).unwrap_or_default();

        let get_prop = |name: &str| -> Option<Variant> {
            map.get(name).cloned()
        };

        let status = get_prop("Status")
            .and_then(|v| v.str().map(|s| s.to_string()))
            .unwrap_or_else(|| "Passive".to_string());

        let title = get_prop("Title")
            .and_then(|v| v.str().map(|s| s.to_string()))
            .unwrap_or_default();

        let icon_name = get_prop("IconName").and_then(|v| v.str().map(|s| s.to_string()));
        let menu_path = get_prop("Menu").and_then(|v| v.str().map(|s| s.to_string()));
        let pixmap = self.pixmap_from_variant(get_prop("IconPixmap"));

        let bus_name = proxy.name().map(|s| s.to_string()).unwrap_or_default();
        let object_path = proxy.object_path().to_string();

        let item = TrayItem {
            identifier: identifier.to_string(),
            bus_name,
            object_path,
            title,
            status,
            icon_name,
            pixmap,
            menu_path,
        };

        self.items.borrow_mut().insert(identifier.to_string(), item);
        self.notify_listeners();
    }

    fn pixmap_from_variant(&self, value: Option<Variant>) -> Option<TrayPixmap> {
        let variant = value?;
        let n_children = variant.n_children();
        if n_children == 0 {
            return None;
        }

        let mut best: Option<(i32, i32, Vec<u8>)> = None;

        for i in 0..n_children {
            let child = variant.child_value(i);
            if child.n_children() < 3 {
                continue;
            }

            let width = child.child_value(0).get::<i32>().unwrap_or(0);
            let height = child.child_value(1).get::<i32>().unwrap_or(0);

            if width <= 0 || height <= 0 {
                continue;
            }

            let data_variant = child.child_value(2);
            let Some(data) = Self::extract_bytes_from_variant(&data_variant) else {
                continue;
            };

            let expected_size = (width as usize) * (height as usize) * 4;
            if data.len() < expected_size {
                continue;
            }

            if best.is_none() || (width * height) > (best.as_ref().unwrap().0 * best.as_ref().unwrap().1) {
                best = Some((width, height, data));
            }
        }

        let (width, height, buffer) = best?;

        Some(TrayPixmap {
            width,
            height,
            buffer: glib::Bytes::from_owned(buffer),
        })
    }

    fn extract_bytes_from_variant(variant: &Variant) -> Option<Vec<u8>> {
        if let Ok(bytes) = variant.fixed_array::<u8>() {
            return Some(bytes.to_vec());
        }
        if variant.type_().is_variant() {
            let inner = variant.child_value(0);
            return Self::extract_bytes_from_variant(&inner);
        }
        None
    }

    pub fn get_menu<F>(&self, identifier: &str, callback: F)
    where
        F: FnOnce(Vec<TrayMenuEntry>) + 'static,
    {
        let identifier = identifier.to_string();
        let (bus_name, menu_path) = {
            let items = self.items.borrow();
            match items.get(&identifier) {
                Some(item) => match &item.menu_path {
                    Some(path) => (item.bus_name.clone(), path.clone()),
                    None => {
                        callback(Vec::new());
                        return;
                    }
                },
                None => {
                    callback(Vec::new());
                    return;
                }
            }
        };

        if let Some(proxy) = self.menu_proxies.borrow().get(&identifier).cloned() {
            Self::fetch_menu_layout(identifier, proxy, callback);
            return;
        }

        let Some(ref _connection) = *self.bus.borrow() else {
            callback(Vec::new());
            return;
        };

        let this_weak = Rc::downgrade(&Self::global());
        let id_clone = identifier.clone();
        gio::DBusProxy::for_bus(
            gio::BusType::Session,
            gio::DBusProxyFlags::DO_NOT_CONNECT_SIGNALS,
            None,
            &bus_name,
            &menu_path,
            "com.canonical.dbusmenu",
            None::<&gio::Cancellable>,
            move |result| {
                let this = match this_weak.upgrade() {
                    Some(t) => t,
                    None => return,
                };
                match result {
                    Ok(proxy) => {
                        this.menu_proxies.borrow_mut().insert(id_clone.clone(), proxy.clone());
                        Self::fetch_menu_layout(id_clone, proxy, callback);
                    }
                    Err(_) => {
                        callback(Vec::new());
                    }
                }
            }
        );
    }

    fn fetch_menu_layout<F>(identifier: String, menu_proxy: gio::DBusProxy, callback: F)
    where
        F: FnOnce(Vec<TrayMenuEntry>) + 'static,
    {
        let _id_clone = identifier.clone();
        let proxy_clone = menu_proxy.clone();
        menu_proxy.call(
            "AboutToShow",
            Some(&(0i32,).to_variant()),
            gio::DBusCallFlags::NONE,
            5000,
            None::<&gio::Cancellable>,
            move |_| {
                let props: Vec<&str> = vec!["label", "enabled", "visible", "type"];
                proxy_clone.call(
                    "GetLayout",
                    Some(&(0i32, -1i32, props).to_variant()),
                    gio::DBusCallFlags::NONE,
                    5000,
                    None::<&gio::Cancellable>,
                    move |result| {
                        let entries = match result {
                            Ok(r) => {
                                let service = Self::global();
                                service.parse_layout_result(&r)
                            }
                            Err(_) => Vec::new(),
                        };
                        callback(entries);
                    }
                );
            }
        );
    }

    fn parse_layout_result(&self, result: &Variant) -> Vec<TrayMenuEntry> {
        if result.n_children() < 2 {
            return Vec::new();
        }
        let layout = result.child_value(1);
        self.parse_layout_node(&layout)
    }

    fn parse_layout_node(&self, node: &Variant) -> Vec<TrayMenuEntry> {
        if node.n_children() < 3 {
            return Vec::new();
        }
        let children_variant = node.child_value(2);
        let mut entries = Vec::new();

        for i in 0..children_variant.n_children() {
            let child = children_variant.child_value(i);
            let actual_child = if child.type_().is_variant() {
                child.child_value(0)
            } else {
                child
            };
            if let Some(entry) = self.node_to_entry(&actual_child) {
                entries.push(entry);
            }
        }
        entries
    }

    fn node_to_entry(&self, node: &Variant) -> Option<TrayMenuEntry> {
        if node.n_children() < 3 {
            return None;
        }

        let menu_id = node.child_value(0).get::<i32>().unwrap_or(0);
        let props = self.parse_menu_properties(&node.child_value(1));

        let visible = props.get("visible").and_then(|v| v.get::<bool>()).unwrap_or(true);
        if !visible {
            return None;
        }

        let entry_type = props.get("type").and_then(|v| v.str()).unwrap_or("");
        if entry_type == "separator" {
            return Some(TrayMenuEntry {
                menu_id,
                label: String::new(),
                enabled: false,
                is_separator: true,
                children: Vec::new(),
            });
        }

        let label = props.get("label").and_then(|v| v.str()).unwrap_or("").to_string();
        let label = label.replace("__", "\u{FFFF}").replace('_', "").replace('\u{FFFF}', "_");

        let enabled = props.get("enabled").and_then(|v| v.get::<bool>()).unwrap_or(true);
        let children = self.parse_layout_node(node);

        Some(TrayMenuEntry {
            menu_id,
            label,
            enabled,
            is_separator: false,
            children,
        })
    }

    fn parse_menu_properties(&self, properties: &Variant) -> HashMap<String, Variant> {
        let mut map = HashMap::new();
        for i in 0..properties.n_children() {
            let entry = properties.child_value(i);
            if entry.n_children() >= 2 {
                if let Some(key) = entry.child_value(0).str() {
                    let val = entry.child_value(1);
                    let actual_val = if val.type_().is_variant() {
                        val.child_value(0)
                    } else {
                        val
                    };
                    map.insert(key.to_string(), actual_val);
                }
            }
        }
        map
    }

    pub fn send_menu_event(&self, identifier: &str, menu_id: i32, event: &str) {
        let menu_proxy = match self.menu_proxies.borrow().get(identifier).cloned() {
            Some(p) => p,
            None => return,
        };

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as u32)
            .unwrap_or(0);
        let data_variant = "".to_variant();
        let params = (menu_id, event, data_variant, timestamp).to_variant();

        menu_proxy.call(
            "Event",
            Some(&params),
            gio::DBusCallFlags::NONE,
            5000,
            None::<&gio::Cancellable>,
            |_| {}
        );
    }
}
