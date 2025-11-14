use gl::types::{GLchar, GLenum, GLint, GLuint};
use std::collections::HashMap;
use std::ffi::{CString, c_void};
use std::ptr::{null, null_mut};
use std::sync::mpsc;
use ul_next::GpuDriver;
use ul_next::bitmap::{BitmapFormat, OwnedBitmap};
use ul_next::gpu_driver::{
    GpuCommand, GpuState, IndexBuffer, RenderBuffer, ShaderType, VertexBuffer, VertexBufferFormat,
};

macro_rules! gl_printiferr {
    () => {
        let err = gl::GetError();
        if err != gl::NO_ERROR {
            eprintln!("OpenGL error: {:04x}", err);
        }
    };
    ($label:expr) => {
        let err = gl::GetError();
        if err != gl::NO_ERROR {
            eprintln!("OpenGL error at {}: {:04x}", $label, err);
        }
    };
}

fn create_uniform_buffer<T>(binding_index: GLuint, data: &T) -> GLuint {
    let mut ubo = 0;
    unsafe {
        gl::GenBuffers(1, &mut ubo);
        gl::BindBuffer(gl::UNIFORM_BUFFER, ubo);
        gl::BufferData(
            gl::UNIFORM_BUFFER,
            std::mem::size_of::<T>() as isize,
            data as *const _ as *const _,
            gl::STATIC_DRAW,
        );
        gl::BindBufferBase(gl::UNIFORM_BUFFER, binding_index, ubo);
        gl::BindBuffer(gl::UNIFORM_BUFFER, 0);
    }
    ubo
}

fn destroy_uniform_buffer(ubo: GLuint) {
    unsafe {
        gl::DeleteBuffers(1, &ubo);
    }
}

fn compile_shader(src: &str, ty: GLenum) -> GLuint {
    unsafe {
        let shader = gl::CreateShader(ty);
        let c_str = CString::new(src.as_bytes()).unwrap();
        gl::ShaderSource(shader, 1, &c_str.as_ptr(), null());
        gl::CompileShader(shader);

        let mut success = gl::FALSE as GLint;
        gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut success);
        if success != gl::TRUE as GLint {
            let mut len = 0;
            gl::GetShaderiv(shader, gl::INFO_LOG_LENGTH, &mut len);
            let mut buffer = vec![0; len as usize];
            buffer.set_len(len as usize - 1);
            gl::GetShaderInfoLog(shader, len, null_mut(), buffer.as_mut_ptr() as *mut GLchar);
            panic!("Shader compile error: {}", String::from_utf8_lossy(&buffer));
        }

        shader
    }
}

fn link_program(vs: GLuint, fs: GLuint) -> GLuint {
    unsafe {
        let program = gl::CreateProgram();
        gl::AttachShader(program, vs);
        gl::AttachShader(program, fs);
        gl::LinkProgram(program);

        let mut success = gl::FALSE as GLint;
        gl::GetProgramiv(program, gl::LINK_STATUS, &mut success);
        if success != gl::TRUE as GLint {
            let mut len = 0;
            gl::GetProgramiv(program, gl::INFO_LOG_LENGTH, &mut len);
            let mut buffer = vec![0; len as usize];
            buffer.set_len(len as usize - 1);
            gl::GetProgramInfoLog(program, len, null_mut(), buffer.as_mut_ptr() as *mut GLchar);
            panic!("Program link error: {}", String::from_utf8_lossy(&buffer));
        }

        gl::DeleteShader(vs);
        gl::DeleteShader(fs);

        program
    }
}

pub enum OpenglCommand {
    CreateTexture(u32, OwnedBitmap),
    UpdateTexture(u32, OwnedBitmap),
    DestroyTexture(u32),
    CreateRenderBuffer(u32, RenderBuffer),
    DestroyRenderBuffer(u32),
    CreateGeometry(u32, VertexBuffer, IndexBuffer),
    UpdateGeometry(u32, VertexBuffer, IndexBuffer),
    DestroyGeometry(u32),
    UpdateCommandList(Vec<GpuCommand>),
}

pub struct OpenglCommandSender {
    next_texture_id: u32,
    next_render_buffer_id: u32,
    next_geometry_id: u32,
    sender: mpsc::Sender<OpenglCommand>,
}

impl OpenglCommandSender {
    pub fn new(sender: mpsc::Sender<OpenglCommand>) -> Self {
        Self {
            next_texture_id: 0,
            next_render_buffer_id: 0,
            next_geometry_id: 0,
            sender,
        }
    }
}

impl GpuDriver for OpenglCommandSender {
    fn begin_synchronize(&mut self) {
        // unhandled
    }

    fn end_synchronize(&mut self) {
        // unhandled
    }

    fn next_texture_id(&mut self) -> u32 {
        self.next_texture_id += 1;
        self.next_texture_id
    }

    fn create_texture(&mut self, texture_id: u32, bitmap: OwnedBitmap) {
        let command = OpenglCommand::CreateTexture(texture_id, bitmap);
        self.sender.send(command).unwrap();
    }

    fn update_texture(&mut self, texture_id: u32, bitmap: OwnedBitmap) {
        let command = OpenglCommand::UpdateTexture(texture_id, bitmap);
        self.sender.send(command).unwrap();
    }

    fn destroy_texture(&mut self, texture_id: u32) {
        let command = OpenglCommand::DestroyTexture(texture_id);
        self.sender.send(command).unwrap();
    }

    fn next_render_buffer_id(&mut self) -> u32 {
        self.next_render_buffer_id += 1;
        self.next_render_buffer_id
    }

    fn create_render_buffer(&mut self, render_buffer_id: u32, render_buffer: RenderBuffer) {
        let command = OpenglCommand::CreateRenderBuffer(render_buffer_id, render_buffer);
        self.sender.send(command).unwrap();
    }

    fn destroy_render_buffer(&mut self, render_buffer_id: u32) {
        let command = OpenglCommand::DestroyRenderBuffer(render_buffer_id);
        self.sender.send(command).unwrap();
    }

    fn next_geometry_id(&mut self) -> u32 {
        self.next_geometry_id += 1;
        self.next_geometry_id
    }

    fn create_geometry(
        &mut self,
        geometry_id: u32,
        vertex_buffer: VertexBuffer,
        index_buffer: IndexBuffer,
    ) {
        let command = OpenglCommand::CreateGeometry(geometry_id, vertex_buffer, index_buffer);
        self.sender.send(command).unwrap();
    }

    fn update_geometry(
        &mut self,
        geometry_id: u32,
        vertex_buffer: VertexBuffer,
        index_buffer: IndexBuffer,
    ) {
        let command = OpenglCommand::UpdateGeometry(geometry_id, vertex_buffer, index_buffer);
        self.sender.send(command).unwrap();
    }

    fn destroy_geometry(&mut self, geometry_id: u32) {
        let command = OpenglCommand::DestroyGeometry(geometry_id);
        self.sender.send(command).unwrap();
    }

    fn update_command_list(&mut self, command_list: Vec<GpuCommand>) {
        let command = OpenglCommand::UpdateCommandList(command_list);
        self.sender.send(command).unwrap();
    }
}

pub struct OpenglCommandReceiver {
    textures: HashMap<u32, GLuint>, // Map texture_id to texture_handle
    render_buffers: HashMap<u32, GLuint>, // Map render_buffer_id to render_buffer_handle
    geometries: HashMap<u32, (GLuint, GLuint, GLuint)>, // Map geometry_id to (VAO, VBO, IBO)
    path_program: GLuint,
    fill_program: GLuint,
    receiver: mpsc::Receiver<OpenglCommand>,
}

impl OpenglCommandReceiver {
    pub fn new(receiver: mpsc::Receiver<OpenglCommand>) -> Self {
        let path_program = link_program(
            compile_shader(
                include_str!("shaders/v2f_c4f_t2f_vert.glsl"),
                gl::VERTEX_SHADER,
            ),
            compile_shader(include_str!("shaders/path_frag.glsl"), gl::FRAGMENT_SHADER),
        );
        let fill_program = link_program(
            compile_shader(
                include_str!("shaders/v2f_c4f_t2f_t2f_d28f_vert.glsl"),
                gl::VERTEX_SHADER,
            ),
            compile_shader(include_str!("shaders/fill_frag.glsl"), gl::FRAGMENT_SHADER),
        );
        Self {
            textures: HashMap::new(),
            render_buffers: HashMap::new(),
            geometries: HashMap::new(),
            path_program,
            fill_program,
            receiver,
        }
    }

    pub fn get_texture_handle(&self, texture_id: u32) -> Option<GLuint> {
        self.textures.get(&texture_id).copied()
    }

    pub fn render(&mut self) {
        while let Ok(command) = self.receiver.try_recv() {
            match command {
                OpenglCommand::CreateTexture(id, bitmap) => self.create_texture(id, bitmap),
                OpenglCommand::UpdateTexture(id, bitmap) => self.update_texture(id, bitmap),
                OpenglCommand::DestroyTexture(id) => self.destroy_texture(id),
                OpenglCommand::CreateRenderBuffer(id, buffer) => {
                    self.create_render_buffer(id, buffer)
                }
                OpenglCommand::DestroyRenderBuffer(id) => self.destroy_render_buffer(id),
                OpenglCommand::CreateGeometry(id, vertex_buffer, index_buffer) => {
                    self.create_geometry(id, vertex_buffer, index_buffer)
                }
                OpenglCommand::UpdateGeometry(id, vertex_buffer, index_buffer) => {
                    self.update_geometry(id, vertex_buffer, index_buffer)
                }
                OpenglCommand::DestroyGeometry(id) => self.destroy_geometry(id),
                OpenglCommand::UpdateCommandList(commands) => self.update_command_list(commands),
            }
        }
    }

    fn create_texture(&mut self, texture_id: u32, bitmap: OwnedBitmap) {
        unsafe {
            let mut id: GLuint = 0;
            gl::GenTextures(1, &mut id);
            let id = id;

            let row_length = bitmap.row_bytes() as i32 / bitmap.bpp() as i32;
            gl::PixelStorei(gl::UNPACK_ALIGNMENT, 1);
            gl::PixelStorei(gl::UNPACK_ROW_LENGTH, row_length);
            gl::PixelStorei(gl::UNPACK_SKIP_ROWS, 0);
            gl::PixelStorei(gl::UNPACK_SKIP_PIXELS, 0);
            gl::PixelStorei(gl::UNPACK_SWAP_BYTES, 0);
            gl::PixelStorei(gl::UNPACK_LSB_FIRST, 0);
            gl::BindTexture(gl::TEXTURE_2D, id);

            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);

            let (internal_format, format, type_) = match bitmap.format() {
                BitmapFormat::A8Unorm => (gl::R8, gl::RED, gl::UNSIGNED_BYTE),
                BitmapFormat::Bgra8UnormSrgb => (gl::SRGB8_ALPHA8, gl::BGRA, gl::UNSIGNED_BYTE),
            };

            let data_ptr = if let Some(ref pixels) = bitmap.pixels() {
                pixels.as_ptr() as *const c_void
            } else {
                null()
            };

            gl::TexImage2D(
                gl::TEXTURE_2D,
                0,
                internal_format as i32,
                bitmap.width() as i32,
                bitmap.height() as i32,
                0,
                format,
                type_,
                data_ptr,
            );

            gl::BindTexture(gl::TEXTURE_2D, 0);

            self.textures.insert(texture_id, id);
        }
    }

    fn update_texture(&mut self, texture_id: u32, bitmap: OwnedBitmap) {
        unsafe {
            if let Some(&id) = self.textures.get(&texture_id) {
                let row_length = bitmap.row_bytes() as i32 / bitmap.bpp() as i32;
                gl::PixelStorei(gl::UNPACK_ALIGNMENT, 1);
                gl::PixelStorei(gl::UNPACK_ROW_LENGTH, row_length);
                gl::PixelStorei(gl::UNPACK_SKIP_ROWS, 0);
                gl::PixelStorei(gl::UNPACK_SKIP_PIXELS, 0);
                gl::PixelStorei(gl::UNPACK_SWAP_BYTES, 0);
                gl::PixelStorei(gl::UNPACK_LSB_FIRST, 0);
                gl::BindTexture(gl::TEXTURE_2D, id);

                let (format, type_) = match bitmap.format() {
                    BitmapFormat::A8Unorm => (gl::RED, gl::UNSIGNED_BYTE),
                    BitmapFormat::Bgra8UnormSrgb => (gl::BGRA, gl::UNSIGNED_BYTE),
                };

                let data_ptr = if let Some(ref pixels) = bitmap.pixels() {
                    pixels.as_ptr() as *const c_void
                } else {
                    null()
                };

                gl::TexSubImage2D(
                    gl::TEXTURE_2D,
                    0,
                    0,
                    0,
                    bitmap.width() as i32,
                    bitmap.height() as i32,
                    format,
                    type_,
                    data_ptr,
                );

                gl::BindTexture(gl::TEXTURE_2D, 0);
            }
        }
    }

    fn destroy_texture(&mut self, texture_id: u32) {
        unsafe {
            if let Some(id) = self.textures.remove(&texture_id) {
                gl::DeleteTextures(1, &id);
            }
        }
    }

    fn create_render_buffer(&mut self, render_buffer_id: u32, render_buffer: RenderBuffer) {
        unsafe {
            let mut fbo: GLuint = 0;
            gl::GenFramebuffers(1, &mut fbo);
            let fbo = fbo;

            gl::BindFramebuffer(gl::FRAMEBUFFER, fbo);
            gl::Enable(gl::FRAMEBUFFER_SRGB);

            let tex = if let Some(&tex) = self.textures.get(&render_buffer.texture_id) {
                tex
            } else {
                panic!("Texture ID {} not found", render_buffer.texture_id);
                // let mut tex: GLuint = 0;
                // gl::GenTextures(1, &mut tex);
                // gl::BindTexture(gl::TEXTURE_2D, tex);

                // gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
                // gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);
                // gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
                // gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);

                // gl::TexImage2D(
                //     gl::TEXTURE_2D,
                //     0,
                //     gl::RGBA8 as i32,
                //     render_buffer.width as i32,
                //     render_buffer.height as i32,
                //     0,
                //     gl::RGBA,
                //     gl::UNSIGNED_BYTE,
                //     std::ptr::null(),
                // );

                // gl::BindTexture(gl::TEXTURE_2D, 0);

                // self.textures.insert(render_buffer.texture_id, tex);
                // tex
            };

            gl::FramebufferTexture2D(
                gl::FRAMEBUFFER,
                gl::COLOR_ATTACHMENT0,
                gl::TEXTURE_2D,
                tex,
                0,
            );

            // if render_buffer.has_depth_buffer && render_buffer.has_stencil_buffer {
            //     let mut rbo: GLuint = 0;
            //     gl::GenRenderbuffers(1, &mut rbo);
            //     gl::BindRenderbuffer(gl::RENDERBUFFER, rbo);
            //     gl::RenderbufferStorage(
            //         gl::RENDERBUFFER,
            //         gl::DEPTH24_STENCIL8,
            //         render_buffer.width as i32,
            //         render_buffer.height as i32,
            //     );
            //     gl::FramebufferRenderbuffer(
            //         gl::FRAMEBUFFER,
            //         gl::DEPTH_STENCIL_ATTACHMENT,
            //         gl::RENDERBUFFER,
            //         rbo,
            //     );
            //     gl::BindRenderbuffer(gl::RENDERBUFFER, 0);
            // } else if render_buffer.has_depth_buffer {
            //     let mut rbo: GLuint = 0;
            //     gl::GenRenderbuffers(1, &mut rbo);
            //     gl::BindRenderbuffer(gl::RENDERBUFFER, rbo);
            //     gl::RenderbufferStorage(
            //         gl::RENDERBUFFER,
            //         gl::DEPTH_COMPONENT24,
            //         render_buffer.width as i32,
            //         render_buffer.height as i32,
            //     );
            //     gl::FramebufferRenderbuffer(
            //         gl::FRAMEBUFFER,
            //         gl::DEPTH_ATTACHMENT,
            //         gl::RENDERBUFFER,
            //         rbo,
            //     );
            //     gl::BindRenderbuffer(gl::RENDERBUFFER, 0);
            // } else if render_buffer.has_stencil_buffer {
            //     let mut rbo: GLuint = 0;
            //     gl::GenRenderbuffers(1, &mut rbo);
            //     gl::BindRenderbuffer(gl::RENDERBUFFER, rbo);
            //     gl::RenderbufferStorage(
            //         gl::RENDERBUFFER,
            //         gl::STENCIL_INDEX8,
            //         render_buffer.width as i32,
            //         render_buffer.height as i32,
            //     );
            //     gl::FramebufferRenderbuffer(
            //         gl::FRAMEBUFFER,
            //         gl::STENCIL_ATTACHMENT,
            //         gl::RENDERBUFFER,
            //         rbo,
            //     );
            //     gl::BindRenderbuffer(gl::RENDERBUFFER, 0);
            // }

            let status = gl::CheckFramebufferStatus(gl::FRAMEBUFFER);
            if status != gl::FRAMEBUFFER_COMPLETE {
                eprintln!(
                    "Incomplete framebuffer (status = 0x{:x}) for render_buffer_id {}",
                    status, render_buffer_id
                );
            }

            gl::BindFramebuffer(gl::FRAMEBUFFER, 0);

            self.render_buffers.insert(render_buffer_id, fbo);
        }
    }

    fn destroy_render_buffer(&mut self, render_buffer_id: u32) {
        unsafe {
            if let Some(id) = self.render_buffers.remove(&render_buffer_id) {
                gl::DeleteFramebuffers(1, &id);
            }
        }
    }

    fn create_geometry(
        &mut self,
        geometry_id: u32,
        vertex_buffer: VertexBuffer,
        index_buffer: IndexBuffer,
    ) {
        unsafe {
            let mut vao: GLuint = 0;
            gl::GenVertexArrays(1, &mut vao);
            let mut vbo: GLuint = 0;
            gl::GenBuffers(1, &mut vbo);
            let mut ibo: GLuint = 0;
            gl::GenBuffers(1, &mut ibo);
            let (vao, vbo, ibo) = (vao, vbo, ibo);

            gl::BindVertexArray(vao);

            // Setup VBO
            gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                vertex_buffer.buffer.len() as isize,
                vertex_buffer.buffer.as_ptr() as *const c_void,
                gl::STATIC_DRAW,
            );

            // Setup vertex attributes
            match vertex_buffer.format {
                VertexBufferFormat::Format_2f_4ub_2f => {
                    let stride = 20;
                    let mut offset = 0;

                    // pos: vec2 (float)
                    gl::EnableVertexAttribArray(0);
                    gl::VertexAttribPointer(0, 2, gl::FLOAT, gl::FALSE, stride, offset as *const _);
                    offset += 2 * std::mem::size_of::<f32>();

                    // color: vec4 (normalized u8)
                    gl::EnableVertexAttribArray(1);
                    gl::VertexAttribPointer(
                        1,
                        4,
                        gl::UNSIGNED_BYTE,
                        gl::TRUE,
                        stride,
                        offset as *const _,
                    );
                    offset += 4 * std::mem::size_of::<u8>();

                    // tex: vec2 (float)
                    gl::EnableVertexAttribArray(2);
                    gl::VertexAttribPointer(2, 2, gl::FLOAT, gl::FALSE, stride, offset as *const _);
                    offset += 2 * std::mem::size_of::<f32>();

                    assert!(
                        offset == stride as usize,
                        "Vertex attribute offset mismatch"
                    );
                }
                VertexBufferFormat::Format_2f_4ub_2f_2f_28f => {
                    let stride = 140;
                    let mut offset = 0;

                    // pos: vec2 (float)
                    gl::EnableVertexAttribArray(0);
                    gl::VertexAttribPointer(0, 2, gl::FLOAT, gl::FALSE, stride, offset as *const _);
                    offset += 2 * std::mem::size_of::<f32>();

                    // color: vec4 (normalized u8)
                    gl::EnableVertexAttribArray(1);
                    gl::VertexAttribPointer(
                        1,
                        4,
                        gl::UNSIGNED_BYTE,
                        gl::TRUE, // normalize 0-255 → 0.0–1.0
                        stride,
                        offset as *const _,
                    );
                    offset += 4 * std::mem::size_of::<u8>();

                    // tex: vec2 (float)
                    gl::EnableVertexAttribArray(2);
                    gl::VertexAttribPointer(2, 2, gl::FLOAT, gl::FALSE, stride, offset as *const _);
                    offset += 2 * std::mem::size_of::<f32>();

                    // obj: vec2 (float)
                    gl::EnableVertexAttribArray(3);
                    gl::VertexAttribPointer(3, 2, gl::FLOAT, gl::FALSE, stride, offset as *const _);
                    offset += 2 * std::mem::size_of::<f32>();

                    // data0 ~ data6 (vec4 float)
                    for i in 0..7 {
                        gl::EnableVertexAttribArray(4 + i);
                        gl::VertexAttribPointer(
                            4 + i as u32,
                            4,
                            gl::FLOAT,
                            gl::FALSE,
                            stride,
                            offset as *const _,
                        );
                        offset += 4 * std::mem::size_of::<f32>();
                    }

                    assert!(
                        offset == stride as usize,
                        "Vertex attribute offset mismatch"
                    );
                }
            }

            // Setup IBO
            gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, ibo);
            gl::BufferData(
                gl::ELEMENT_ARRAY_BUFFER,
                (index_buffer.buffer.len() * std::mem::size_of::<u32>()) as isize,
                index_buffer.buffer.as_ptr() as *const c_void,
                gl::STATIC_DRAW,
            );

            gl::BindVertexArray(0);

            self.geometries.insert(geometry_id, (vao, vbo, ibo));
        }
    }

    fn update_geometry(
        &mut self,
        geometry_id: u32,
        vertex_buffer: VertexBuffer,
        index_buffer: IndexBuffer,
    ) {
        unsafe {
            if let Some(&(vao, vbo, ibo)) = self.geometries.get(&geometry_id) {
                gl::BindVertexArray(vao);

                // Update VBO
                gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
                gl::BufferSubData(
                    gl::ARRAY_BUFFER,
                    0,
                    vertex_buffer.buffer.len() as isize,
                    vertex_buffer.buffer.as_ptr() as *const c_void,
                );

                // Update IBO
                gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, ibo);
                gl::BufferSubData(
                    gl::ELEMENT_ARRAY_BUFFER,
                    0,
                    (index_buffer.buffer.len() * std::mem::size_of::<u32>()) as isize,
                    index_buffer.buffer.as_ptr() as *const c_void,
                );

                gl::BindVertexArray(0);
            }
        }
    }

    fn destroy_geometry(&mut self, geometry_id: u32) {
        unsafe {
            if let Some((vao, vbo, ibo)) = self.geometries.remove(&geometry_id) {
                gl::DeleteBuffers(1, &vbo);
                gl::DeleteBuffers(1, &ibo);
                gl::DeleteVertexArrays(1, &vao);
            }
        }
    }

    fn update_command_list(&mut self, command_list: Vec<GpuCommand>) {
        for command in command_list {
            // println!("Command: {:?}", command);
            match command {
                GpuCommand::ClearRenderBuffer { render_buffer_id } => {
                    self.clear_render_buffer(render_buffer_id)
                }
                GpuCommand::DrawGeometry {
                    gpu_state,
                    geometry_id,
                    indices_offset,
                    indices_count,
                } => self.draw_geometry(gpu_state, geometry_id, indices_offset, indices_count),
            }
        }
    }

    fn clear_render_buffer(&mut self, render_buffer_id: u32) {
        unsafe {
            if let Some(&id) = self.render_buffers.get(&render_buffer_id) {
                gl::BindFramebuffer(gl::FRAMEBUFFER, id);
                gl::ClearColor(0.0, 0.0, 0.0, 0.0);
                gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
                gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
            } else {
                panic!("Render buffer ID {} not found", render_buffer_id);
            }
        }
    }

    fn draw_geometry(
        &mut self,
        gpu_state: Box<GpuState>,
        geometry_id: u32,
        indices_offset: u32,
        indices_count: u32,
    ) {
        unsafe {
            let &id = self
                .render_buffers
                .get(&gpu_state.render_buffer_id)
                .unwrap();
            let (width, height) = (
                gpu_state.viewport_width as i32,
                gpu_state.viewport_height as i32,
            );

            gl::BindFramebuffer(gl::FRAMEBUFFER, id);
            gl::Viewport(0, 0, width, height);

            let status = gl::CheckFramebufferStatus(gl::FRAMEBUFFER);
            if status != gl::FRAMEBUFFER_COMPLETE {
                eprintln!("Framebuffer incomplete: 0x{:X}", status);
            }

            // 混色
            if gpu_state.enable_blend {
                gl::Enable(gl::BLEND);
                gl::BlendFuncSeparate(
                    gl::SRC_ALPHA,
                    gl::ONE_MINUS_SRC_ALPHA,
                    gl::ONE,
                    gl::ONE_MINUS_SRC_ALPHA,
                );
            } else {
                gl::Disable(gl::BLEND);
            }

            // 裁剪
            if gpu_state.enable_scissor {
                gl::Enable(gl::SCISSOR_TEST);
                let r = &gpu_state.scissor_rect;
                gl::Scissor(r.left, r.top, r.right - r.left, r.bottom - r.top);
            } else {
                gl::Disable(gl::SCISSOR_TEST);
            }

            // 绑定着色器程序
            let program = match gpu_state.shader_type {
                ShaderType::Fill => self.fill_program,
                ShaderType::FillPath => self.path_program,
            };
            gl::UseProgram(program);

            // 纹理绑定
            let [tex1, tex2, tex3] = [
                gpu_state.texture_1_id,
                gpu_state.texture_2_id,
                gpu_state.texture_3_id,
            ]
            .map(|o| {
                o.map(|id| self.textures.get(&id))
                    .flatten()
                    .copied()
                    .unwrap_or(0)
            });
            gl::ActiveTexture(gl::TEXTURE1);
            gl::BindTexture(gl::TEXTURE_2D, tex1);
            gl::ActiveTexture(gl::TEXTURE2);
            gl::BindTexture(gl::TEXTURE_2D, tex2);
            gl::ActiveTexture(gl::TEXTURE3);
            gl::BindTexture(gl::TEXTURE_2D, tex3);

            let tex1_loc = gl::GetUniformLocation(program, b"Texture1\0".as_ptr() as *const _);
            let tex2_loc = gl::GetUniformLocation(program, b"Texture2\0".as_ptr() as *const _);
            let tex3_loc = gl::GetUniformLocation(program, b"Texture3\0".as_ptr() as *const _);

            gl::Uniform1i(tex1_loc, 1);
            gl::Uniform1i(tex2_loc, 2);
            gl::Uniform1i(tex3_loc, 3);

            let &(vao, _, _) = self.geometries.get(&geometry_id).unwrap();
            gl::BindVertexArray(vao);

            // Orthographic Projection matrix applied to
            // the `transformation` matrix.
            let orth_projection_matrix = [
                [2.0 / gpu_state.viewport_width as f32, 0.0, 0.0, 0.0],
                [0.0, 2.0 / gpu_state.viewport_height as f32, 0.0, 0.0],
                [0.0, 0.0, -0.000002, 0.0],
                [-1.0, -1.0, 0.818183, 1.0],
            ];
            // trasform matrix to project matrix
            let mut transformation = [
                [0., 0., 0., 0.],
                [0., 0., 0., 0.],
                [0., 0., 0., 0.],
                [0., 0., 0., 0.],
            ];

            // multiply matrices
            for i in 0..4 {
                for j in 0..4 {
                    for k in 0..4 {
                        transformation[i][j] +=
                            gpu_state.transform[i * 4 + k] * orth_projection_matrix[k][j];
                    }
                }
            }

            let transform_loc =
                gl::GetUniformLocation(program, b"Transform\0".as_ptr() as *const _);
            gl::UniformMatrix4fv(
                transform_loc,
                1,
                gl::FALSE,
                transformation.as_ptr() as *const f32,
            );

            let state = [
                0.0,
                gpu_state.viewport_width as f32,
                gpu_state.viewport_height as f32,
                1.0,
            ];

            let state_loc = gl::GetUniformLocation(program, b"State\0".as_ptr() as *const _);
            gl::Uniform4fv(state_loc, 1, state.as_ptr());

            let clip_loc = gl::GetUniformLocation(program, b"ClipSize\0".as_ptr() as *const _);
            gl::Uniform1ui(clip_loc, gpu_state.clip_size as u32);

            let scalar_ubo = create_uniform_buffer(0, &gpu_state.uniform_scalar);
            let vector_ubo = create_uniform_buffer(1, &gpu_state.uniform_vector);
            let clip_ubo = create_uniform_buffer(2, &gpu_state.clip);

            let scalar_idx = gl::GetUniformBlockIndex(program, b"Scalar\0".as_ptr() as *const _);
            let vector_idx = gl::GetUniformBlockIndex(program, b"Vector\0".as_ptr() as *const _);
            let clip_idx = gl::GetUniformBlockIndex(program, b"Clip\0".as_ptr() as *const _);

            gl::UniformBlockBinding(program, scalar_idx, 0);
            gl::UniformBlockBinding(program, vector_idx, 1);
            gl::UniformBlockBinding(program, clip_idx, 2);

            gl::DrawElements(
                gl::TRIANGLES,
                indices_count as i32,
                gl::UNSIGNED_INT,
                (indices_offset as usize * std::mem::size_of::<u32>()) as *const c_void,
            );

            gl_printiferr!();

            gl::UseProgram(0);

            destroy_uniform_buffer(scalar_ubo);
            destroy_uniform_buffer(vector_ubo);
            destroy_uniform_buffer(clip_ubo);

            gl::BindVertexArray(0);
            gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
        }
    }
}

pub fn create_gpu_driver() -> (OpenglCommandSender, OpenglCommandReceiver) {
    let (sender, receiver) = mpsc::channel();
    let command_sender = OpenglCommandSender::new(sender);
    let command_receiver = OpenglCommandReceiver::new(receiver);
    (command_sender, command_receiver)
}
