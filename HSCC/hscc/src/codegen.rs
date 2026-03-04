use alloc::string::String;
use crate::ast::*;
use std::collections::HashMap;

pub struct CodeGenerator {
    output: String,
    indent: usize,
    kernel_count: usize,
    var_map: HashMap<String, String>, // 变量名 -> C++ 类型声明
}

impl CodeGenerator {
    pub fn new() -> Self {
        CodeGenerator {
            output: String::new(),
            indent: 0,
            kernel_count: 0,
            var_map: HashMap::new(),
        }
    }

    fn emit(&mut self, s: &str) {
        self.output.push_str(s);
    }

    fn emitln(&mut self, s: &str) {
        self.output.push_str(&" ".repeat(self.indent));
        self.output.push_str(s);
        self.output.push('\n');
    }

    fn indent_inc(&mut self) {
        self.indent += 1;
    }

    fn indent_dec(&mut self) {
        debug_assert!(self.indent > 0, "indent underflow: mismatched indent_inc/indent_dec");
        self.indent = self.indent.saturating_sub(1);
    }

    pub fn generate(&mut self, program: &Program) -> String {
        self.emitln("#include <cuda_runtime.h>");
        self.emitln("#include <stdio.h>");
        self.emitln("#include <stdlib.h>");
        self.emitln("#include <vector>");
        self.emitln("#include <cstring>");
        self.emitln("");
        self.emitln("// 设备常量");
        self.emitln("const int GPU = 0;");
        self.emitln("const int CPU = -1;");
        self.emitln("const int NPU = 1;");
        self.emitln("const int FPGA = 2;");
        self.emitln("const int Host = -1;");
        self.emitln("");
        self.emitln("// Buffer 封装，支持多维形状和主机/设备位置");
        self.emitln("template<typename T>");
        self.emitln("struct Buffer {");
        self.indent_inc();
        self.emitln("T* data;");
        self.emitln("size_t size;        // 总元素个数");
        self.emitln("int device;          // -1 表示主机，>=0 表示设备 ID");
        self.emitln("std::vector<size_t> dims; // 各维度大小");
        self.emitln("");
        self.emitln("Buffer() : data(nullptr), size(0), device(-1) {}");
        self.emitln("~Buffer() { if (data) { if (device == -1) free(data); else cudaFree(data); } }");
        self.emitln("Buffer(const Buffer&) = delete;");
        self.emitln("Buffer& operator=(const Buffer&) = delete;");
        self.emitln("Buffer(Buffer&& other) : data(other.data), size(other.size), device(other.device), dims(std::move(other.dims)) { other.data = nullptr; }");
        self.emitln("Buffer& operator=(Buffer&& other) { if (this != &other) { if (data) { if (device == -1) free(data); else cudaFree(data); } data = other.data; size = other.size; device = other.device; dims = std::move(other.dims); other.data = nullptr; } return *this; }");
        self.emitln("");
        self.emitln("static Buffer zeros(std::initializer_list<size_t> shape) {");
        self.indent_inc();
        self.emitln("Buffer buf;");
        self.emitln("buf.dims = shape;");
        self.emitln("buf.size = 1;");
        self.emitln("for (auto d : shape) buf.size *= d;");
        self.emitln("buf.data = (T*)malloc(buf.size * sizeof(T)); // 默认在主机");
        self.emitln("memset(buf.data, 0, buf.size * sizeof(T));");
        self.emitln("buf.device = -1;");
        self.emitln("return buf;");
        self.indent_dec();
        self.emitln("}");
        self.emitln("");
        self.emitln("Buffer& place_on(int dev) {");
        self.indent_inc();
        self.emitln("this->device = dev; // 仅设置设备标记，不迁移");
        self.emitln("return *this;");
        self.indent_dec();
        self.emitln("}");
        self.emitln("");
        self.emitln("Buffer& move_to(int dev) {");
        self.indent_inc();
        self.emitln("if (dev == this->device) return *this;");
        self.emitln("T* new_ptr;");
        self.emitln("if (dev == -1) { // 目标为主机");
        self.indent_inc();
        self.emitln("new_ptr = (T*)malloc(size * sizeof(T));");
        self.emitln("cudaMemcpy(new_ptr, data, size * sizeof(T), cudaMemcpyDeviceToHost);");
        self.emitln("cudaFree(data);");
        self.indent_dec();
        self.emitln("} else { // 目标为设备");
        self.indent_inc();
        self.emitln("cudaMalloc(&new_ptr, size * sizeof(T));");
        self.emitln("if (this->device == -1) {");
        self.indent_inc();
        self.emitln("cudaMemcpy(new_ptr, data, size * sizeof(T), cudaMemcpyHostToDevice);");
        self.emitln("free(data);");
        self.indent_dec();
        self.emitln("} else {");
        self.indent_inc();
        self.emitln("cudaMemcpy(new_ptr, data, size * sizeof(T), cudaMemcpyDeviceToDevice);");
        self.emitln("cudaFree(data);");
        self.indent_dec();
        self.emitln("}");
        self.indent_dec();
        self.emitln("}");
        self.emitln("this->data = new_ptr;");
        self.emitln("this->device = dev;");
        self.emitln("return *this;");
        self.indent_dec();
        self.emitln("}");
        self.emitln("");
        self.emitln("const size_t* shape() const { return dims.data(); }");
        self.emitln("size_t ndim() const { return dims.size(); }");
        self.emitln("T& operator[](size_t i) { return data[i]; }");
        self.emitln("const T& operator[](size_t i) const { return data[i]; }");
        self.indent_dec();
        self.emitln("};");
        self.emitln("");

        // 生成 kernel 声明
        for task in &program.tasks {
            self.generate_task_kernel(task);
        }

        // 生成主机函数
        for func in &program.functions {
            self.generate_function(func);
        }

        self.output.clone()
    }

    fn generate_task_kernel(&mut self, task: &Task) {
        // 内核名称
        let kernel_name = format!("{}_kernel", task.name);
        // 从任务参数中提取 Buffer 类型参数名和维度
        // 假设参数为 a: Buffer<f32>, b: Buffer<f32>，返回 Buffer<f32>
        // 生成内核参数：指针 + 维度
        self.emitln(&format!("__global__ void {}(float* a, float* b, float* c, int M, int K, int N) {{", kernel_name));
        self.indent_inc();
        
        // 遍历任务体中的 parallel for 语句
        for stmt in &task.body.statements {
            if let Statement::ParallelFor { var, range: _, body } = stmt {
                // 生成线程索引
                self.emitln(&format!("int {} = blockIdx.x * blockDim.x + threadIdx.x;", var));
                self.emitln(&format!("if ({} < M) {{", var));
                self.indent_inc();
                // 生成内部循环体
                for inner_stmt in &body.statements {
                    self.generate_statement(inner_stmt);
                }
                self.indent_dec();
                self.emitln("}");
            } else {
                // 其他语句直接生成
                self.generate_statement(stmt);
            }
        }
        
        self.indent_dec();
        self.emitln("}");
        self.emitln("");
    }

    fn generate_function(&mut self, func: &Function) {
        // 生成函数返回类型
        let return_type = match &func.return_type {
            Some(Type::F32) => "float",
            Some(Type::F64) => "double",
            Some(Type::I32) => "int",
            Some(Type::I64) => "long long",
            Some(Type::Bool) => "bool",
            Some(Type::Buffer(_, _)) => "Buffer<float>",
            _ => "void",
        };

        // 生成参数字符串
        let params: Vec<String> = func.params.iter().map(|param| {
            let param_type = match param.ty {
                Type::F32 => "float",
                Type::F64 => "double",
                Type::I32 => "int",
                Type::I64 => "long long",
                Type::Bool => "bool",
                Type::Buffer(_, _) => "Buffer<float>",
                _ => "auto",
            };
            format!("{} {}", param_type, param.name)
        }).collect::<Vec<_>>();
        let param_str = params.join(", ");

        // 判断是否为 main 函数
        if func.name == "main" {
            self.emitln("int main() {");
        } else {
            self.emitln(&format!("{} {}({}) {{", return_type, func.name, param_str));
        }

        self.indent_inc();

        // 生成函数体
        for stmt in &func.body.statements {
            self.generate_statement(stmt);
        }

        // 如果不是 main 函数且具有返回类型，确保有 return 语句
        if func.name != "main" {
            // 检查最后一个语句是否是 return, 如果不是则添加默认 return
            if let Some(last_stmt) = func.body.statements.last() {
                if !matches!(last_stmt, Statement::Return(_)) {
                   if func.return_type.is_some() {
                       self.emitln("return;");
                   }
                }
            }
        } else {
            self.emitln("return 0;");
        }

        self.indent_dec();
        self.emitln("}");
        self.emitln("");
    }

    fn generate_statement(&mut self, stmt: &Statement) {
        match stmt {
            Statement::Let { mutable: _, name, ty: _ty, init } => {
                // 变量声明
                if let Some(expr) = init {
                    self.emit(&format!("auto {} = ", name));
                    self.generate_expression(expr);
                    self.emitln(";");
                } else {
                    // 仅声明，未初始化
                }
            }
            Statement::Expr(expr) => {
                self.generate_expression(expr);
                self.emitln(";");
            }
            Statement::Return(expr) => {
                self.emit("return ");
                if let Some(e) = expr {
                    self.generate_expression(e);
                }
                self.emitln(";");
            }
            Statement::ParallelFor { var: _var, range: _range, body: _body } => {
                // 忽略，已经在 kernel 中实现
            }
            Statement::For { var, range, body } => {
                self.emit(&format!("for (int {} = ", var));
                self.generate_expression(&range.0);
                self.emit("; ");
                self.emit(&format!("{} < ", var));
                self.generate_expression(&range.1);
                self.emit("; ");
                self.emit(&format!("{}++) ", var));
                self.emitln("{");
                self.indent_inc();
                for stmt in &body.statements {
                    self.generate_statement(stmt);
                }
                self.indent_dec();
                self.emitln("}");
            }
            _ => {}
        }
    }
    
    fn generate_expression(&mut self, expr: &Expression) { 
        match expr { 
            Expression::Integer(i) => self.emit(&i.to_string()),
            Expression::Float(f) => self.emit(&f.to_string()),
            Expression::String(s) => self.emit(&format!("\"{}\"", s)),
            Expression::Bool(b) => self.emit(if *b { "true" } else { "false" }),
            Expression::Nil => self.emit("nullptr"),
            Expression::Identifier(id) => self.emit(id),
            Expression::Path(path) => {
                // 生成路径，例如 hsc::*
                for (i, segment) in path.segments.iter().enumerate() {
                    if i > 0 {
                        self.emit("::");
                    }
                    self.emit(&segment.ident);
                    if let Some(generic_args) = &segment.generic_args {
                        self.emit("<");
                        for (j, arg) in generic_args.iter().enumerate() {
                            if j > 0 {
                                self.emit(", ");
                            }
                            // 根据泛型参数的实际类型生成对应的 C++ 类型
                            self.generate_generic_arg(arg);
                        }
                        self.emit(">");
                    }
                }
            }
            Expression::Binary { left, op, right } => {
                self.emit("(");
                self.generate_expression(left);
                self.emit(match op { 
                    BinaryOp::Add => " + ",
                    BinaryOp::Sub => " - ",
                    BinaryOp::Mul => " * ",
                    BinaryOp::Div => " / ",
                    BinaryOp::Eq => " == ",
                    BinaryOp::Ne => " != ",
                    BinaryOp::Lt => " < ",
                    BinaryOp::Le => " <= ",
                    BinaryOp::Gt => " > ",
                    BinaryOp::Ge => " >= ",
                    BinaryOp::And => " && ",
                    BinaryOp::Or => " || ",
                });
                self.generate_expression(right);
                self.emit(")");
            }
            Expression::Call { func, args } => {
                // 特殊处理内置函数
                let func_name = match func.as_ref() {
                    Expression::Path(path) => path.segments.last().unwrap().ident.as_str(),
                    Expression::Identifier(id) => id.as_str(),
                    _ => "",
                };
                if func_name == "log!" {
                    // 生成 printf
                    self.emit("printf(");
                    self.generate_expression(&args[0]);
                    for arg in &args[1..] {
                        self.emit(", ");
                        self.generate_expression(arg);
                    }
                    self.emit(")");
                    return;
                } else if func_name == "save_output" {
                    // 生成文件写入
                    self.emitln("{");
                    self.indent_inc();
                    self.emit("const char* path = ");
                    self.generate_expression(&args[0]);
                    self.emitln(";");
                    self.emit("Buffer<float> data = ");
                    self.generate_expression(&args[1]);
                    self.emitln(";");
                    self.emitln("FILE* f = fopen(path, \"wb\");");
                    self.emitln("if (f) {");
                    self.indent_inc();
                    self.emitln("fwrite(data.data, sizeof(float), data.size, f);");
                    self.emitln("fclose(f);");
                    self.indent_dec();
                    self.emitln("}");
                    self.indent_dec();
                    self.emit("}");
                    return;
                }

                // 普通函数调用
                self.generate_expression(func);
                self.emit("(");
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        self.emit(", ");
                    }
                    self.generate_expression(arg);
                }
                self.emit(")");
            }
            Expression::FieldAccess { obj, field } => {
                self.generate_expression(obj);
                self.emit(&format!(".{}", field));
            }
            Expression::Index { obj, index } => {
                self.generate_expression(obj);
                self.emit("[");
                self.generate_expression(index);
                self.emit("]");
            }
            Expression::MethodCall { obj, method, args } => {
                self.generate_expression(obj);
                self.emit(&format!(".{}(", method));
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        self.emit(", ");
                    }
                    self.generate_expression(arg);
                }
                self.emit(")");
            }
            Expression::PlaceOn { expr, device } => {
                self.generate_expression(expr);
                self.emit(".place_on(");
                self.generate_expression(device);
                self.emit(")");
            }
            Expression::MoveTo { expr, device } => {
                self.generate_expression(expr);
                self.emit(".move_to(");
                self.generate_expression(device);
                self.emit(")");
            }
            Expression::Await(expr) => {
                self.generate_expression(expr);
                // 在 spawn 中处理 await
            }
            Expression::Array(elems) => {
                self.emit("{");
                for (i, elem) in elems.iter().enumerate() {
                    if i > 0 {
                        self.emit(", ");
                    }
                    self.generate_expression(elem);
                }
                self.emit("}");
            }
            Expression::Spawn { device, task, await_ } => {
                if let Expression::Call { func, args } = task.as_ref() {
                    if let Expression::Path(path) = func.as_ref() {
                        let task_name = path.segments.last().unwrap().ident.clone();
                        self.emit("[&]() -> Buffer<float> {");
                        self.indent_inc();
                        self.emitln("");
                        // 提取维度
                        self.emit("int M = ");
                        self.generate_expression(&args[0]);
                        self.emitln(".shape()[0];");
                        self.emit("int K = ");
                        self.generate_expression(&args[0]);
                        self.emitln(".shape()[1];");
                        self.emit("int N = ");
                        self.generate_expression(&args[1]);
                        self.emitln(".shape()[1];");
                        // 分配结果缓冲区
                        self.emitln("Buffer<float> c = Buffer<float>::zeros({M, N});");
                        if let Some(dev_expr) = device {
                            self.emit("c = c.move_to(");
                            self.generate_expression(dev_expr);
                            self.emitln(");");
                        }
                        // 获取指针
                        self.emit("float* a_ptr = ");
                        self.generate_expression(&args[0]);
                        self.emitln(".data;");
                        self.emit("float* b_ptr = ");
                        self.generate_expression(&args[1]);
                        self.emitln(".data;");
                        self.emit("float* c_ptr = c.data;");
                        // 启动内核
                        self.emitln("dim3 block(16, 16);");
                        self.emitln("dim3 grid((M + 15) / 16, (N + 15) / 16);");
                        self.emitln(&format!("{}_kernel<<<grid, block>>>(a_ptr, b_ptr, c_ptr, M, K, N);", task_name));
                        if *await_ {
                            self.emitln("cudaDeviceSynchronize();");
                        }
                        self.emitln("return c;");
                        self.indent_dec();
                        self.emit("}()");
                    } else {
                        // 错误处理
                        self.emit("/* invalid spawn task */");
                    }
                } else {
                    self.emit("/* invalid spawn call */");
                }
            }
        }
    }

    fn generate_generic_arg(&mut self, arg: &Type) {
        match arg {
            Type::F32 => self.emit("float"),
            Type::F64 => self.emit("double"),
            Type::I32 => self.emit("int"),
            Type::I64 => self.emit("long long"),
            Type::Bool => self.emit("bool"),
            Type::Buffer(elem_type, _) => {
                // 递归处理 Buffer 的元素类型
                self.emit("Buffer<");
                self.generate_generic_arg(elem_type);
                self.emit(">");
            }
            Type::Named(name) => {
                self.emit(name);
            }
            Type::Tuple(_) => self.emit("auto"),
            _ => self.emit("auto"),
        }
    }
}