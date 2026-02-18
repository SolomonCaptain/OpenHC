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
        self.indent -= 1;
    }

    pub fn generate(&mut self, program: &Program) -> String {
        self.emitln("#include <cuda_runtime.h>");
        self.emitln("#include <stdio.h>");
        self.emitln("#include <stdlib.h>");
        self.emitln("");
        self.emitln("// Buffer 简单封装");
        self.emitln("template<typename T>");
        self.emitln("struct Buffer {");
        self.indent_inc();
        self.emitln("T* data;");
        self.emitln("size_t size;");
        self.emitln("int device;");
        self.emitln("");
        self.emitln("__host__ __device__ Buffer() : data(nullptr), size(0), device(-1) {}");
        self.emitln("__host__ __device__ Buffer(T* d, size_t s, int dev) : data(d), size(s), device(dev) {}");
        self.emitln("");
        self.emitln("static Buffer zero(size_t n) {");
        self.indent_inc();
        self.emitln("T* h_ptr = (T*)malloc(n * sizeof(T), cudaMemcpyHostToDevice);");
        self.emitln("free(h_ptr);");
        self.emitln("return Buffer(d_ptr, n, 0); // device 0");
        self.indent_dec();
        self.emitln("}");
        self.emitln("");
        self.emitln("Buffer place_on(int dev) {");
        self.indent_inc();
        self.emitln("// 简单实现：仅设置设备标记，实际不迁移");
        self.emitln("this->device = dev;");
        self.emitln("return *this;");
        self.indent_dec();
        self.emitln("}");
        self.emitln("");
        self.emitln("Buffer move_to(int dev) {");
        self.indent_inc();
        self.emitln("if (dev == this->device) return *this;");
        self.emitln("T* new_ptr == nullptr;");
        self.emitln("cudaMalloc(&new_ptr, size * sizeof(T));");
        self.emitln("cudaMemcpy(new_ptr, data, size * sizeof(T), cudaMemcpyDeviceToDevice);");
        self.emitln("cudaFree(data);");
        self.emitln("this->data = new_ptr;");
        self.emitln("this->device = dev;");
        self.emitln("return *this;");
        self.indent_dec();
        self.emitln("}");
        self.emitln("");
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
        let kernel_name = format!("{}_kernel", task.name);
        self.emitln(&format!("__global__ void {}(float* a, float* b, float* c, int M, int K, int N) {{", kernel_name));
        self.indent_inc();
        self.emitln("int i = blockIdx.x * blockDim.x + threadIdx.x;");
        self.emitln("int j = blockIdx.y * blockDim.y + threadIdx.y;");
        self.emitln("if (i < M && j < N) {");
        self.indent_inc();
        self.emitln("float sum = 0.0f;");
        self.emitln("for (int l = 0; l < K; ++l) {");
        self.indent_inc();
        self.emitln("sum += a[i * K + l] * b[l * N + j];");
        self.indent_dec();
        self.emitln("}");
        self.emitln("c[i * N + j] = sum;");
        self.indent_dec();
        self.emitln("}");
        self.indent_dec();
        self.emitln("}");
        self.emitln("");
    }

    fn generate_function(&mut self, func: &Function) {
        self.emitln(&format!("int main() {{"));
        self.indent_inc();

        // 生成函数体
        for stmt in &func.body.statements {
            self.generate_statement(stmt);
        }

        self.emitln("return 0;");
        self.indent_dec();
        self.emitln("}");
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
                self.emitln("return ");
                if let Some(e) = expr {
                    self.generate_expression(e);
                }
                self.emitln(";");
            }
            Statement::Spawn { device: _device, task, await_ } => {
                // 解析任务调用
                if let Expression::Call { func, args } = task {
                    if let Expression::Identifier(task_name) = func.as_ref() {
                        if task_name == "gpu_matmul" {
                            // 假设 args: a, b
                            let a_expr = &args[0];
                            let b_expr = &args[1];
                            self.emit("// Spawn task\n");
                            self.emit("Buffer<float> c;\n");
                            self.emit("{\n");
                            self.indent_inc();
                            self.emit("float* a_ptr = ");
                            self.generate_expression(a_expr);
                            self.emitln(".data;");
                            self.emit("float* b_ptr = ");
                            self.generate_expression(b_expr);
                            self.emitln(".data;");
                            self.emitln("int M = 1024; int K = 1024; int N = 1024; // 固定尺寸");
                            self.emitln("cudaMalloc(&c.data, M*N*sizeof(float));");
                            self.emitln("c.size = M*N;");
                            self.emitln("dim3 block(16,16);");
                            self.emitln("dim3 grid((M+15)/16, (N+15)/16);");
                            self.emitln(&format!("{}_kernel<<<grid, block>>>(a_ptr, b_ptr, c.data, M, K, N);", task_name));
                            if *await_ {
                                self.emitln("cudaDeviceSynchronize();");
                            }
                            self.indent_dec();
                            self.emitln("}");
                        }
                    }
                }
            }
            Statement::ParallelFor { var: _var, range: _range, body: _body } => {
                // 忽略，已经在 kernel 中实现
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
        }
    }
}