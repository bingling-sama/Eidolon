use glium::backend::glutin::Display;
use glium::glutin::surface::WindowSurface;

// Define a type alias for Display with the WindowSurface type parameter
pub type GliumDisplay = Display<WindowSurface>;
