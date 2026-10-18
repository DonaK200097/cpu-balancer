// Без окна консоли в релизе
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    cpu_balancer_lib::run()
}
