#[macro_use]
extern crate glium;

mod app;
mod constants;
mod model;
mod renderer;
mod texture;
mod utils;

use app::App;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 创建应用程序实例
    let app = App::new()?;

    // 运行应用程序
    app.run()
}
