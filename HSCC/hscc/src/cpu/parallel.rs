//! CPU 并行化模块
//!
//! 提供 OpenMP 并行化和多线程支持

use crate::ast::*;

/// 线程调度策略
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ThreadSchedule {
    /// 静态调度（均匀分配）
    Static,
    /// 动态调度（按需分配）
    Dynamic,
    /// 引导式调度（逐渐减小块大小）
    Guided,
    /// 自动调度（运行时决定）
    Auto,
    /// 运行时调度（由环境变量决定）
    Runtime,
}

impl ThreadSchedule {
    /// 转换为 OpenMP 调度字符串
    pub fn to_omp_string(&self) -> &'static str {
        match self {
            ThreadSchedule::Static => "static",
            ThreadSchedule::Dynamic => "dynamic",
            ThreadSchedule::Guided => "guided",
            ThreadSchedule::Auto => "auto",
            ThreadSchedule::Runtime => "runtime",
        }
    }
}

/// 并行配置
#[derive(Debug, Clone)]
pub struct ParallelConfig {
    /// 是否启用 OpenMP
    pub enable_openmp: bool,
    /// 线程数（0 表示自动）
    pub num_threads: usize,
    /// 调度策略
    pub schedule: ThreadSchedule,
    /// 块大小（用于静态/动态调度）
    pub chunk_size: usize,
    /// 是否启用 SIMD
    pub enable_simd: bool,
    /// 是否启用嵌套并行
    pub enable_nested: bool,
}

impl Default for ParallelConfig {
    fn default() -> Self {
        ParallelConfig {
            enable_openmp: true,
            num_threads: 0,
            schedule: ThreadSchedule::Static,
            chunk_size: 0,
            enable_simd: true,
            enable_nested: false,
        }
    }
}

impl ParallelConfig {
    /// 创建新的并行配置
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置线程数
    pub fn with_threads(mut self, threads: usize) -> Self {
        self.num_threads = threads;
        self
    }

    /// 设置调度策略
    pub fn with_schedule(mut self, schedule: ThreadSchedule) -> Self {
        self.schedule = schedule;
        self
    }

    /// 设置块大小
    pub fn with_chunk_size(mut self, chunk: usize) -> Self {
        self.chunk_size = chunk;
        self
    }

    /// 禁用 OpenMP
    pub fn without_openmp(mut self) -> Self {
        self.enable_openmp = false;
        self
    }

    /// 禁用 SIMD
    pub fn without_simd(mut self) -> Self {
        self.enable_simd = false;
        self
    }
}

/// 并行化分析结果
#[derive(Debug, Clone)]
pub struct ParallelAnalysis {
    /// 循环是否可并行化
    pub is_parallelizable: bool,
    /// 并行化原因（如果不可并行化）
    pub reason: Option<String>,
    /// 建议的线程数
    pub suggested_threads: usize,
    /// 建议的块大小
    pub suggested_chunk: usize,
    /// 是否存在数据竞争
    pub has_data_race: bool,
    /// 归约变量列表
    pub reduction_vars: Vec<String>,
}

impl ParallelAnalysis {
    pub fn parallelizable() -> Self {
        ParallelAnalysis {
            is_parallelizable: true,
            reason: None,
            suggested_threads: 0,
            suggested_chunk: 0,
            has_data_race: false,
            reduction_vars: vec![],
        }
    }

    pub fn not_parallelizable(reason: &str) -> Self {
        ParallelAnalysis {
            is_parallelizable: false,
            reason: Some(reason.to_string()),
            suggested_threads: 0,
            suggested_chunk: 0,
            has_data_race: false,
            reduction_vars: vec![],
        }
    }
}

/// 并行化优化器
pub struct Parallelizer {
    /// 并行配置
    config: ParallelConfig,
}

impl Parallelizer {
    /// 创建新的并行化优化器
    pub fn new(config: ParallelConfig) -> Self {
        Parallelizer { config }
    }

    /// 使用默认配置创建
    pub fn with_default_config() -> Self {
        Parallelizer::new(ParallelConfig::default())
    }

    /// 分析循环是否可并行化
    pub fn analyze_loop(&self, var: &str, body: &[Statement]) -> ParallelAnalysis {
        // 简化的并行化分析
        // 实际实现需要更复杂的数据依赖分析

        let mut analysis = ParallelAnalysis::parallelizable();

        // 检查循环体中的语句
        for stmt in body {
            if !self.is_statement_parallelizable(stmt, var) {
                return ParallelAnalysis::not_parallelizable(
                    &format!("Statement depends on loop iteration order")
                );
            }

            // 检查归约变量
            if let Statement::Expr(expr) = stmt {
                if let Some((var_name, _)) = self.is_reduction_expression(expr) {
                    analysis.reduction_vars.push(var_name);
                }
            }
        }

        // 计算建议的线程数和块大小
        analysis.suggested_threads = self.config.num_threads;
        analysis.suggested_chunk = self.calculate_chunk_size(body.len());

        analysis
    }

    /// 检查语句是否可并行化
    fn is_statement_parallelizable(&self, stmt: &Statement, loop_var: &str) -> bool {
        match stmt {
            Statement::Let { init, .. } => {
                // 变量声明通常是安全的
                if let Some(expr) = init {
                    self.is_expression_parallelizable(expr, loop_var)
                } else {
                    true
                }
            }
            Statement::Expr(expr) => {
                self.is_expression_parallelizable(expr, loop_var)
            }
            Statement::If { condition, then_branch, else_branch } => {
                self.is_expression_parallelizable(condition, loop_var)
                    && then_branch.statements.iter().all(|s| self.is_statement_parallelizable(s, loop_var))
                    && else_branch.as_ref()
                        .map(|b| b.statements.iter().all(|s| self.is_statement_parallelizable(s, loop_var)))
                        .unwrap_or(true)
            }
            Statement::For { var, body, .. } | Statement::ParallelFor { var, body, .. } => {
                body.statements.iter().all(|s| self.is_statement_parallelizable(s, var))
            }
            Statement::While { condition, body } => {
                self.is_expression_parallelizable(condition, loop_var)
                    && body.statements.iter().all(|s| self.is_statement_parallelizable(s, loop_var))
            }
            Statement::Loop(body) => {
                body.statements.iter().all(|s| self.is_statement_parallelizable(s, loop_var))
            }
            Statement::Return(_) | Statement::Break | Statement::Continue => true,
        }
    }

    /// 检查表达式是否可并行化
    fn is_expression_parallelizable(&self, expr: &Expression, _loop_var: &str) -> bool {
        // 简化版本：假设大多数表达式都是可并行化的
        match expr {
            Expression::Binary { left, right, .. } => {
                self.is_expression_parallelizable(left, _loop_var)
                    && self.is_expression_parallelizable(right, _loop_var)
            }
            Expression::Call { func, args } => {
                self.is_expression_parallelizable(func, _loop_var)
                    && args.iter().all(|a| self.is_expression_parallelizable(a, _loop_var))
            }
            Expression::Index { obj, index } => {
                self.is_expression_parallelizable(obj, _loop_var)
                    && self.is_expression_parallelizable(index, _loop_var)
            }
            Expression::MethodCall { obj, args, .. } => {
                self.is_expression_parallelizable(obj, _loop_var)
                    && args.iter().all(|a| self.is_expression_parallelizable(a, _loop_var))
            }
            _ => true,
        }
    }

    /// 检查是否是归约表达式，返回 (变量名, 操作)
    fn is_reduction_expression(&self, expr: &Expression) -> Option<(String, BinaryOp)> {
        match expr {
            Expression::Binary { op, left, right } => {
                // 检查是否是 x = x + y 形式
                if let Expression::Identifier(name) = left.as_ref() {
                    // 简化：假设这是归约模式
                    if matches!(op, BinaryOp::Add | BinaryOp::Mul | BinaryOp::Sub) {
                        return Some((name.clone(), *op));
                    }
                }
                None
            }
            Expression::Call { func, args } => {
                // 检查函数调用是否是归约
                if let Expression::Identifier(func_name) = func.as_ref() {
                    if func_name == "reduce" && !args.is_empty() {
                        if let Expression::Identifier(var_name) = &args[0] {
                            return Some((var_name.clone(), BinaryOp::Add));
                        }
                    }
                }
                None
            }
            _ => None,
        }
    }

    /// 计算建议的块大小
    fn calculate_chunk_size(&self, body_size: usize) -> usize {
        if self.config.chunk_size > 0 {
            return self.config.chunk_size;
        }

        // 自动计算块大小
        let threads = if self.config.num_threads > 0 {
            self.config.num_threads
        } else {
            8  // 默认假设 8 个线程
        };

        (body_size / threads).max(1)
    }

    /// 生成 OpenMP 指令
    pub fn generate_omp_directive(&self, analysis: &ParallelAnalysis) -> String {
        if !self.config.enable_openmp || !analysis.is_parallelizable {
            return String::new();
        }

        let schedule = self.config.schedule.to_omp_string();
        let mut directive = format!("#pragma omp parallel for schedule({}", schedule);

        if self.config.chunk_size > 0 {
            directive.push_str(&format!(", {}", self.config.chunk_size));
        }

        directive.push(')');

        // 添加归约子句
        if !analysis.reduction_vars.is_empty() {
            for var in &analysis.reduction_vars {
                directive.push_str(&format!(" reduction(+:{})", var));
            }
        }

        // 添加线程数
        if self.config.num_threads > 0 {
            directive.push_str(&format!(" num_threads({})", self.config.num_threads));
        }

        directive
    }

    /// 生成 SIMD 指令
    pub fn generate_simd_directive(&self) -> String {
        if self.config.enable_simd {
            "#pragma omp simd".to_string()
        } else {
            String::new()
        }
    }

    /// 生成并行区域
    pub fn generate_parallel_region(&self, body: &str) -> String {
        if !self.config.enable_openmp {
            return body.to_string();
        }

        let mut result = String::new();

        result.push_str("#pragma omp parallel");
        if self.config.num_threads > 0 {
            result.push_str(&format!(" num_threads({})", self.config.num_threads));
        }
        result.push_str("\n{\n");
        result.push_str(body);
        result.push_str("\n}\n");

        result
    }

    /// 生成临界区
    pub fn generate_critical_section(&self, body: &str, name: Option<&str>) -> String {
        if !self.config.enable_openmp {
            return body.to_string();
        }

        let mut result = String::new();

        result.push_str("#pragma omp critical");
        if let Some(n) = name {
            result.push_str(&format!("({})", n));
        }
        result.push_str("\n{\n");
        result.push_str(body);
        result.push_str("\n}\n");

        result
    }

    /// 生成原子操作
    pub fn generate_atomic_update(&self, var: &str, op: &str, value: &str) -> String {
        if !self.config.enable_openmp {
            return format!("{} {}= {};", var, op, value);
        }

        format!("#pragma omp atomic\n{} {}= {};", var, op, value)
    }

    /// 获取配置
    pub fn config(&self) -> &ParallelConfig {
        &self.config
    }
}

impl Default for Parallelizer {
    fn default() -> Self {
        Self::with_default_config()
    }
}

/// 循环展开优化
pub struct LoopUnroller {
    /// 展开因子
    unroll_factor: usize,
}

impl LoopUnroller {
    pub fn new(factor: usize) -> Self {
        LoopUnroller { unroll_factor: factor }
    }

    /// 使用默认因子
    pub fn default_factor() -> Self {
        LoopUnroller::new(4)
    }

    /// 分析是否应该展开
    pub fn should_unroll(&self, body_size: usize, trip_count: Option<usize>) -> bool {
        // 小循环体和已知的小迭代次数适合展开
        if body_size < 10 {
            if let Some(count) = trip_count {
                return count <= self.unroll_factor * 4;
            }
        }
        false
    }

    /// 生成展开指令
    pub fn generate_unroll_directive(&self) -> String {
        format!("#pragma unroll {}", self.unroll_factor)
    }
}

impl Default for LoopUnroller {
    fn default() -> Self {
        Self::default_factor()
    }
}

/// 向量化优化
pub struct Vectorizer {
    /// 目标向量宽度
    vector_width: usize,
}

impl Vectorizer {
    pub fn new(width: usize) -> Self {
        Vectorizer { vector_width: width }
    }

    /// 使用默认 AVX 宽度
    pub fn avx() -> Self {
        Vectorizer { vector_width: 8 }
    }

    /// 使用默认 SSE 宽度
    pub fn sse() -> Self {
        Vectorizer { vector_width: 4 }
    }

    /// 生成向量化指令
    pub fn generate_vector_directive(&self) -> String {
        format!("#pragma omp simd simdlen({})", self.vector_width)
    }

    /// 检查是否可以向量化
    pub fn can_vectorize(&self, stmt: &Statement) -> bool {
        // 简化的向量化检查
        match stmt {
            Statement::Let { init, .. } => {
                init.as_ref()
                    .map(|e| self.is_expression_vectorizable(e))
                    .unwrap_or(true)
            }
            Statement::Expr(expr) => self.is_expression_vectorizable(expr),
            _ => false,
        }
    }

    /// 检查表达式是否可以向量化
    fn is_expression_vectorizable(&self, expr: &Expression) -> bool {
        match expr {
            Expression::Binary { op, left, right } => {
                // 大多数算术操作可以向量化
                matches!(op, BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div)
                    && self.is_expression_vectorizable(left)
                    && self.is_expression_vectorizable(right)
            }
            Expression::Index { obj, index } => {
                self.is_expression_vectorizable(obj)
                    && self.is_expression_vectorizable(index)
            }
            Expression::Identifier(_) => true,
            Expression::Integer(_) | Expression::Float(_) => true,
            _ => false,
        }
    }
}

impl Default for Vectorizer {
    fn default() -> Self {
        Self::avx()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parallel_config() {
        let config = ParallelConfig::default()
            .with_threads(4)
            .with_schedule(ThreadSchedule::Dynamic);

        assert_eq!(config.num_threads, 4);
        assert_eq!(config.schedule, ThreadSchedule::Dynamic);
    }

    #[test]
    fn test_thread_schedule() {
        assert_eq!(ThreadSchedule::Static.to_omp_string(), "static");
        assert_eq!(ThreadSchedule::Dynamic.to_omp_string(), "dynamic");
    }

    #[test]
    fn test_parallelizer() {
        let parallelizer = Parallelizer::with_default_config();

        let body = vec![
            Statement::Expr(Expression::Integer(1))
        ];

        let analysis = parallelizer.analyze_loop("i", &body);
        assert!(analysis.is_parallelizable);
    }

    #[test]
    fn test_loop_unroller() {
        let unroller = LoopUnroller::new(4);
        assert!(unroller.should_unroll(5, Some(8)));
        assert!(!unroller.should_unroll(20, Some(100)));
    }

    #[test]
    fn test_vectorizer() {
        let vectorizer = Vectorizer::avx();
        let stmt = Statement::Expr(Expression::Binary {
            left: Box::new(Expression::Integer(1)),
            op: BinaryOp::Add,
            right: Box::new(Expression::Integer(2)),
        });
        assert!(vectorizer.can_vectorize(&stmt));
    }
}
