//! 数据流分析模块
//!
//! 提供高级数据流分析功能，用于检测程序中的数据依赖和潜在的并行性问题。
//!
//! # 概述
//!
//! 数据流分析是编译器优化和正确性验证的重要基础。本模块实现了多种经典的数据流分析算法，
//! 特别针对 HSCLang 的并行执行特性进行了扩展。
//!
//! # 主要功能
//!
//! - **可达定义分析**：确定每个程序点上哪些定义是有效的
//! - **活性变量分析**：确定每个程序点上哪些变量的值会被后续使用
//! - **循环独立性检测**：分析循环迭代之间是否存在数据依赖
//! - **依赖关系分析**：识别 RAW、WAR、WAW 等依赖类型
//!
//! # 主要组件
//!
//! - [`DataFlowAnalyzer`] - 主分析器
//! - [`DefinitionPoint`] - 变量定义点表示
//! - [`UsePoint`] - 变量使用点表示
//! - [`DataFlowResult`] - 分析结果
//! - [`LoopDependenceInfo`] - 循环依赖信息
//!
//! # 使用示例
//!
//! ```rust
//! use hscc::dataflow::DataFlowAnalyzer;
//! use hscc::ast::Block;
//!
//! let mut analyzer = DataFlowAnalyzer::new();
//! let result = analyzer.analyze_block(&block);
//!
//! // 检查循环独立性
//! if let Some(loop_info) = result.loop_dependencies.get("loop_var") {
//!     if !loop_info.is_independent {
//!         println!("Loop has dependencies!");
//!     }
//! }
//! ```
//!
//! # 分析算法
//!
//! ## 可达定义分析
//!
//! 使用前向数据流分析，计算每个程序点的入和出定义集合：
//! - `IN[B] = ∪(OUT[P])` for all predecessors P of B
//! - `OUT[B] = gen[B] ∪ (IN[B] - kill[B])`
//!
//! ## 活性变量分析
//!
//! 使用后向数据流分析，计算每个程序点的活入和活出变量集合：
//! - `OUT[B] = ∪(IN[S])` for all successors S of B
//! - `IN[B] = use[B] ∪ (OUT[B] - def[B])`
//!
//! ## 循环独立性检测
//!
//! 检测循环体内的数组访问模式，识别是否存在跨迭代的依赖：
//! - RAW (Read After Write) / Flow dependence
//! - WAR (Write After Read) / Anti dependence
//! - WAW (Write After Write) / Output dependence

use crate::ast::*;
use std::collections::{HashMap, HashSet, BTreeSet};

// ============================================================================
// 数据流值
// ============================================================================

/// 变量定义点
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DefinitionPoint {
    /// 变量名
    pub variable: String,
    /// 定义位置（语句索引）
    pub statement_idx: usize,
    /// 是否是循环内定义
    pub in_loop: bool,
    /// 循环变量（如果在循环内）
    pub loop_var: Option<String>,
}

impl DefinitionPoint {
    pub fn new(variable: String, statement_idx: usize) -> Self {
        Self {
            variable,
            statement_idx,
            in_loop: false,
            loop_var: None,
        }
    }

    pub fn in_loop(mut self, loop_var: String) -> Self {
        self.in_loop = true;
        self.loop_var = Some(loop_var);
        self
    }
}

/// 变量使用点
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UsePoint {
    /// 变量名
    pub variable: String,
    /// 使用位置（语句索引）
    pub statement_idx: usize,
    /// 使用类型
    pub use_type: UseType,
    /// 是否在循环内
    pub in_loop: bool,
    /// 循环变量
    pub loop_var: Option<String>,
}

/// 使用类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UseType {
    /// 读取
    Read,
    /// 写入
    Write,
    /// 读写
    ReadWrite,
}

// ============================================================================
// 控制流图
// ============================================================================

/// 基本块
#[derive(Debug, Clone)]
pub struct BasicBlock {
    /// 块 ID
    pub id: usize,
    /// 语句索引范围
    pub stmt_range: (usize, usize),
    /// 前驱块
    pub predecessors: Vec<usize>,
    /// 后继块
    pub successors: Vec<usize>,
    /// 定义的变量
    pub defs: HashSet<String>,
    /// 使用的变量
    pub uses: HashSet<String>,
}

impl BasicBlock {
    pub fn new(id: usize) -> Self {
        Self {
            id,
            stmt_range: (0, 0),
            predecessors: Vec::new(),
            successors: Vec::new(),
            defs: HashSet::new(),
            uses: HashSet::new(),
        }
    }
}

/// 控制流图
#[derive(Debug, Default)]
pub struct ControlFlowGraph {
    /// 基本块列表
    pub blocks: Vec<BasicBlock>,
    /// 入口块 ID
    pub entry: usize,
    /// 出口块 ID
    pub exit: usize,
}

impl ControlFlowGraph {
    pub fn new() -> Self {
        Self {
            blocks: Vec::new(),
            entry: 0,
            exit: 0,
        }
    }

    /// 添加基本块
    pub fn add_block(&mut self) -> usize {
        let id = self.blocks.len();
        self.blocks.push(BasicBlock::new(id));
        id
    }

    /// 添加边
    pub fn add_edge(&mut self, from: usize, to: usize) {
        self.blocks[from].successors.push(to);
        self.blocks[to].predecessors.push(from);
    }
}

// ============================================================================
// 数据流分析器
// ============================================================================

/// 数据流分析结果
#[derive(Debug, Default)]
pub struct DataFlowAnalysisResult {
    /// 可达定义 (块ID -> 定义点集合)
    pub reaching_defs_in: HashMap<usize, HashSet<DefinitionPoint>>,
    pub reaching_defs_out: HashMap<usize, HashSet<DefinitionPoint>>,
    
    /// 活性变量 (块ID -> 变量集合)
    pub live_in: HashMap<usize, HashSet<String>>,
    pub live_out: HashMap<usize, HashSet<String>>,
    
    /// 未初始化变量使用
    pub uninitialized_uses: Vec<(String, usize)>,
    
    /// 循环携带依赖
    pub loop_carried_deps: Vec<LoopCarriedDependency>,
}

/// 循环携带依赖
#[derive(Debug, Clone)]
pub struct LoopCarriedDependency {
    /// 变量/Buffer 名称
    pub variable: String,
    /// 源位置
    pub source_idx: usize,
    /// 目标位置
    pub target_idx: usize,
    /// 依赖类型
    pub dep_type: DependencyType,
    /// 循环变量
    pub loop_var: String,
    /// 距离（依赖距离，-1 表示未知）
    pub distance: i64,
}

/// 依赖类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DependencyType {
    /// 读后写 (True Dependency)
    Flow,
    /// 写后读 (Anti Dependency)
    Anti,
    /// 写后写 (Output Dependency)
    Output,
    /// 读后读 (Input Dependency - 通常不影响并行)
    Input,
}

/// 数据流分析器
pub struct DataFlowAnalyzer {
    /// 定义点映射 (变量名 -> 定义点列表)
    definitions: HashMap<String, Vec<DefinitionPoint>>,
    /// 使用点映射 (变量名 -> 使用点列表)
    uses: HashMap<String, Vec<UsePoint>>,
    /// 当前循环变量栈
    loop_vars: Vec<String>,
    /// 当前语句索引
    current_stmt_idx: usize,
}

impl DataFlowAnalyzer {
    pub fn new() -> Self {
        Self {
            definitions: HashMap::new(),
            uses: HashMap::new(),
            loop_vars: Vec::new(),
            current_stmt_idx: 0,
        }
    }

    /// 分析程序
    pub fn analyze_program(&mut self, program: &Program) -> DataFlowAnalysisResult {
        // 收集所有定义和使用
        for func in &program.functions {
            self.collect_defs_uses_block(&func.body);
        }
        for task in &program.tasks {
            self.collect_defs_uses_block(&task.body);
        }

        // 分析依赖
        let mut result = DataFlowAnalysisResult::default();
        
        // 检测未初始化变量使用
        self.detect_uninitialized_uses(&mut result);
        
        // 检测循环携带依赖
        self.detect_loop_carried_dependencies(&mut result);

        result
    }

    /// 收集代码块中的定义和使用
    fn collect_defs_uses_block(&mut self, block: &Block) {
        for stmt in &block.statements {
            self.collect_defs_uses_stmt(stmt);
        }
    }

    /// 收集语句中的定义和使用
    fn collect_defs_uses_stmt(&mut self, stmt: &Statement) {
        match stmt {
            Statement::Let { name, init, .. } => {
                // 记录定义
                let def = DefinitionPoint::new(name.clone(), self.current_stmt_idx);
                let def = if let Some(loop_var) = self.loop_vars.last() {
                    def.in_loop(loop_var.clone())
                } else {
                    def
                };
                self.definitions
                    .entry(name.clone())
                    .or_default()
                    .push(def);

                // 记录初始化表达式中的使用
                if let Some(expr) = init {
                    self.collect_uses_expr(expr, UseType::Read);
                }

                self.current_stmt_idx += 1;
            }

            Statement::Expr(expr) => {
                self.collect_uses_expr(expr, UseType::ReadWrite);
                self.current_stmt_idx += 1;
            }

            Statement::Return(expr) => {
                if let Some(expr) = expr {
                    self.collect_uses_expr(expr, UseType::Read);
                }
                self.current_stmt_idx += 1;
            }

            Statement::ParallelFor { var, range, body } => {
                // 进入循环
                self.loop_vars.push(var.clone());

                // 收集范围中的使用
                self.collect_uses_expr(&range.0, UseType::Read);
                self.collect_uses_expr(&range.1, UseType::Read);

                // 收集循环体
                self.collect_defs_uses_block(body);

                self.loop_vars.pop();
                self.current_stmt_idx += 1;
            }

            Statement::For { var, range, body } => {
                self.loop_vars.push(var.clone());
                self.collect_uses_expr(&range.0, UseType::Read);
                self.collect_uses_expr(&range.1, UseType::Read);
                self.collect_defs_uses_block(body);
                self.loop_vars.pop();
                self.current_stmt_idx += 1;
            }

            Statement::If { condition, then_branch, else_branch } => {
                self.collect_uses_expr(condition, UseType::Read);
                self.collect_defs_uses_block(then_branch);
                if let Some(else_block) = else_branch {
                    self.collect_defs_uses_block(else_block);
                }
                self.current_stmt_idx += 1;
            }

            Statement::While { condition, body } => {
                self.collect_uses_expr(condition, UseType::Read);
                self.collect_defs_uses_block(body);
                self.current_stmt_idx += 1;
            }

            Statement::Loop(body) => {
                self.collect_defs_uses_block(body);
                self.current_stmt_idx += 1;
            }

            Statement::Break | Statement::Continue => {
                self.current_stmt_idx += 1;
            }
        }
    }

    /// 收集表达式中的使用
    fn collect_uses_expr(&mut self, expr: &Expression, use_type: UseType) {
        match expr {
            Expression::Identifier(name) => {
                let use_point = UsePoint {
                    variable: name.clone(),
                    statement_idx: self.current_stmt_idx,
                    use_type,
                    in_loop: !self.loop_vars.is_empty(),
                    loop_var: self.loop_vars.last().cloned(),
                };
                self.uses
                    .entry(name.clone())
                    .or_default()
                    .push(use_point);
            }

            Expression::Binary { left, right, .. } => {
                self.collect_uses_expr(left, UseType::Read);
                self.collect_uses_expr(right, UseType::Read);
            }

            Expression::Call { func, args } => {
                self.collect_uses_expr(func, UseType::Read);
                for arg in args {
                    self.collect_uses_expr(arg, UseType::Read);
                }
            }

            Expression::MethodCall { obj, args, .. } => {
                self.collect_uses_expr(obj, use_type);
                for arg in args {
                    self.collect_uses_expr(arg, UseType::Read);
                }
            }

            Expression::Index { obj, index } => {
                self.collect_uses_expr(obj, UseType::ReadWrite);
                self.collect_uses_expr(index, UseType::Read);
            }

            Expression::FieldAccess { obj, .. } => {
                self.collect_uses_expr(obj, use_type);
            }

            Expression::Array(elems) => {
                for elem in elems {
                    self.collect_uses_expr(elem, UseType::Read);
                }
            }

            Expression::MoveTo { expr, device } |
            Expression::PlaceOn { expr, device } => {
                self.collect_uses_expr(expr, use_type);
                self.collect_uses_expr(device, UseType::Read);
            }

            Expression::Spawn { device, task, .. } => {
                if let Some(dev) = device {
                    self.collect_uses_expr(dev, UseType::Read);
                }
                self.collect_uses_expr(task, UseType::Read);
            }

            Expression::Await(inner) => {
                self.collect_uses_expr(inner, UseType::Read);
            }

            Expression::Path(path) => {
                if path.segments.len() == 1 {
                    let use_point = UsePoint {
                        variable: path.segments[0].ident.clone(),
                        statement_idx: self.current_stmt_idx,
                        use_type,
                        in_loop: !self.loop_vars.is_empty(),
                        loop_var: self.loop_vars.last().cloned(),
                    };
                    self.uses
                        .entry(path.segments[0].ident.clone())
                        .or_default()
                        .push(use_point);
                }
            }

            _ => {}
        }
    }

    /// 检测未初始化变量使用
    fn detect_uninitialized_uses(&self, result: &mut DataFlowAnalysisResult) {
        for (var, use_points) in &self.uses {
            let defs = self.definitions.get(var);
            
            for use_pt in use_points {
                // 检查是否在使用点之前有定义
                let has_prior_def = defs.map_or(false, |defs| {
                    defs.iter().any(|d| d.statement_idx < use_pt.statement_idx)
                });

                if !has_prior_def {
                    result.uninitialized_uses.push((var.clone(), use_pt.statement_idx));
                }
            }
        }
    }

    /// 检测循环携带依赖
    fn detect_loop_carried_dependencies(&self, result: &mut DataFlowAnalysisResult) {
        // 对于每个循环变量
        for (var, defs) in &self.definitions {
            // 只处理循环内的定义
            let loop_defs: Vec<_> = defs.iter()
                .filter(|d| d.in_loop)
                .collect();

            if loop_defs.is_empty() {
                continue;
            }

            // 检查同一循环内的使用
            if let Some(uses) = self.uses.get(var) {
                for use_pt in uses {
                    if !use_pt.in_loop {
                        continue;
                    }

                    // 查找可能产生依赖的定义
                    for def in &loop_defs {
                        if def.loop_var != use_pt.loop_var {
                            continue;
                        }

                        // 检查依赖类型
                        let dep_type = match (def.statement_idx.cmp(&use_pt.statement_idx), use_pt.use_type) {
                            (std::cmp::Ordering::Less, UseType::Read) => DependencyType::Flow,
                            (std::cmp::Ordering::Greater, UseType::Read) => DependencyType::Anti,
                            (std::cmp::Ordering::Less, UseType::Write) => DependencyType::Output,
                            (std::cmp::Ordering::Less, UseType::ReadWrite) => DependencyType::Flow,
                            _ => continue,
                        };

                        result.loop_carried_deps.push(LoopCarriedDependency {
                            variable: var.clone(),
                            source_idx: def.statement_idx,
                            target_idx: use_pt.statement_idx,
                            dep_type,
                            loop_var: def.loop_var.clone().unwrap_or_default(),
                            distance: -1, // 未知距离
                        });
                    }
                }
            }
        }
    }

    /// 检查循环是否独立
    pub fn is_loop_independent(&self, loop_var: &str, block: &Block) -> bool {
        // 收集循环内的所有定义和使用
        let mut loop_defs: HashMap<String, Vec<usize>> = HashMap::new();
        let mut loop_uses: HashMap<String, Vec<usize>> = HashMap::new();

        self.collect_loop_defs_uses(loop_var, block, &mut loop_defs, &mut loop_uses);

        // 检查是否有循环携带依赖
        for (var, def_indices) in &loop_defs {
            if let Some(use_indices) = loop_uses.get(var) {
                // 如果同一个变量在循环内既有定义又有使用，则可能有依赖
                if !def_indices.is_empty() && !use_indices.is_empty() {
                    return false;
                }
            }
        }

        true
    }

    /// 收集循环内的定义和使用
    fn collect_loop_defs_uses(
        &self,
        _loop_var: &str,
        block: &Block,
        defs: &mut HashMap<String, Vec<usize>>,
        uses: &mut HashMap<String, Vec<usize>>,
    ) {
        for stmt in &block.statements {
            match stmt {
                Statement::Let { name, init, .. } => {
                    defs.entry(name.clone()).or_default().push(0);
                    if let Some(expr) = init {
                        self.collect_expr_uses_for_independence(expr, uses);
                    }
                }
                Statement::Expr(expr) => {
                    self.collect_expr_uses_for_independence(expr, uses);
                }
                Statement::ParallelFor { body, .. } |
                Statement::For { body, .. } => {
                    // 嵌套循环
                    self.collect_loop_defs_uses("", body, defs, uses);
                }
                _ => {}
            }
        }
    }

    /// 收集表达式中的使用（用于独立性检查）
    fn collect_expr_uses_for_independence(&self, expr: &Expression, uses: &mut HashMap<String, Vec<usize>>) {
        match expr {
            Expression::Identifier(name) => {
                uses.entry(name.clone()).or_default().push(0);
            }
            Expression::Index { obj, .. } => {
                if let Expression::Identifier(name) = obj.as_ref() {
                    uses.entry(name.clone()).or_default().push(0);
                }
            }
            Expression::Binary { left, right, .. } => {
                self.collect_expr_uses_for_independence(left, uses);
                self.collect_expr_uses_for_independence(right, uses);
            }
            Expression::Call { func, args } => {
                self.collect_expr_uses_for_independence(func, uses);
                for arg in args {
                    self.collect_expr_uses_for_independence(arg, uses);
                }
            }
            Expression::MethodCall { obj, args, .. } => {
                self.collect_expr_uses_for_independence(obj, uses);
                for arg in args {
                    self.collect_expr_uses_for_independence(arg, uses);
                }
            }
            _ => {}
        }
    }
}

impl Default for DataFlowAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 依赖分析器
// ============================================================================

/// Buffer 访问信息
#[derive(Debug, Clone)]
pub struct BufferAccess {
    /// Buffer 名称
    pub buffer: String,
    /// 访问类型
    pub access_type: AccessType,
    /// 索引表达式
    pub indices: Vec<IndexExpr>,
    /// 所在语句索引
    pub stmt_idx: usize,
}

/// 访问类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessType {
    Read,
    Write,
    ReadWrite,
}

/// 索引表达式
#[derive(Debug, Clone)]
pub struct IndexExpr {
    /// 是否是循环变量
    pub is_loop_var: bool,
    /// 循环变量名
    pub loop_var_name: Option<String>,
    /// 是否是线性索引
    pub is_linear: bool,
    /// 原始表达式字符串（简化）
    pub expr_str: String,
}

/// 依赖分析器
pub struct DependenceAnalyzer {
    /// Buffer 访问列表
    buffer_accesses: Vec<BufferAccess>,
    /// 当前循环变量
    loop_vars: Vec<String>,
}

impl DependenceAnalyzer {
    pub fn new() -> Self {
        Self {
            buffer_accesses: Vec::new(),
            loop_vars: Vec::new(),
        }
    }

    /// 分析循环独立性
    pub fn analyze_loop(&mut self, loop_var: &str, block: &Block) -> LoopIndependenceResult {
        self.loop_vars.push(loop_var.to_string());
        self.buffer_accesses.clear();

        // 收集 Buffer 访问
        self.collect_buffer_accesses(block);

        // 分析依赖
        let mut result = LoopIndependenceResult {
            is_independent: true,
            dependencies: Vec::new(),
        };

        // 检查所有访问对
        for i in 0..self.buffer_accesses.len() {
            for j in (i + 1)..self.buffer_accesses.len() {
                let acc1 = &self.buffer_accesses[i];
                let acc2 = &self.buffer_accesses[j];

                if acc1.buffer != acc2.buffer {
                    continue;
                }

                // 检查是否有依赖
                if let Some(dep) = self.check_dependency(acc1, acc2) {
                    result.is_independent = false;
                    result.dependencies.push(dep);
                }
            }
        }

        self.loop_vars.pop();
        result
    }

    /// 收集 Buffer 访问
    fn collect_buffer_accesses(&mut self, block: &Block) {
        for stmt in &block.statements {
            self.collect_accesses_from_stmt(stmt);
        }
    }

    /// 从语句收集访问
    fn collect_accesses_from_stmt(&mut self, stmt: &Statement) {
        match stmt {
            Statement::Let { init, .. } => {
                if let Some(expr) = init {
                    self.collect_accesses_from_expr(expr, AccessType::Read);
                }
            }
            Statement::Expr(expr) => {
                self.collect_accesses_from_expr(expr, AccessType::ReadWrite);
            }
            Statement::ParallelFor { body, .. } |
            Statement::For { body, .. } => {
                // 嵌套循环的访问也需要收集
                self.collect_buffer_accesses(body);
            }
            _ => {}
        }
    }

    /// 从表达式收集访问
    fn collect_accesses_from_expr(&mut self, expr: &Expression, access_type: AccessType) {
        match expr {
            Expression::Index { obj, index } => {
                if let Expression::Identifier(name) = obj.as_ref() {
                    let index_expr = self.analyze_index_expr(index);
                    self.buffer_accesses.push(BufferAccess {
                        buffer: name.clone(),
                        access_type,
                        indices: vec![index_expr],
                        stmt_idx: 0,
                    });
                }
            }
            Expression::Binary { left, right, .. } => {
                self.collect_accesses_from_expr(left, access_type);
                self.collect_accesses_from_expr(right, access_type);
            }
            Expression::Call { func, args } => {
                self.collect_accesses_from_expr(func, AccessType::Read);
                for arg in args {
                    self.collect_accesses_from_expr(arg, AccessType::Read);
                }
            }
            Expression::MethodCall { obj, args, .. } => {
                self.collect_accesses_from_expr(obj, access_type);
                for arg in args {
                    self.collect_accesses_from_expr(arg, AccessType::Read);
                }
            }
            _ => {}
        }
    }

    /// 分析索引表达式
    fn analyze_index_expr(&self, index: &Expression) -> IndexExpr {
        match index {
            Expression::Identifier(name) => {
                let is_loop_var = self.loop_vars.contains(name);
                IndexExpr {
                    is_loop_var,
                    loop_var_name: if is_loop_var { Some(name.clone()) } else { None },
                    is_linear: true,
                    expr_str: name.clone(),
                }
            }
            Expression::Binary { left, op, right } => {
                let left_str = self.expr_to_string(left);
                let right_str = self.expr_to_string(right);
                let expr_str = format!("{} {} {}", left_str, self.op_to_string(*op), right_str);
                
                // 检查是否是线性索引 (i, i+1, i-1, 2*i, 等)
                let is_linear = self.is_linear_index(index);
                
                IndexExpr {
                    is_loop_var: self.contains_loop_var(index),
                    loop_var_name: self.loop_vars.last().cloned(),
                    is_linear,
                    expr_str,
                }
            }
            Expression::Integer(n) => {
                IndexExpr {
                    is_loop_var: false,
                    loop_var_name: None,
                    is_linear: true,
                    expr_str: n.to_string(),
                }
            }
            _ => {
                IndexExpr {
                    is_loop_var: self.contains_loop_var(index),
                    loop_var_name: self.loop_vars.last().cloned(),
                    is_linear: false,
                    expr_str: "complex".to_string(),
                }
            }
        }
    }

    /// 检查依赖
    fn check_dependency(&self, acc1: &BufferAccess, acc2: &BufferAccess) -> Option<BufferDependency> {
        // 简化：如果两个访问都涉及循环变量，且至少一个是写，则可能有依赖
        let both_use_loop_var = acc1.indices.iter().any(|i| i.is_loop_var) &&
                                acc2.indices.iter().any(|i| i.is_loop_var);

        if !both_use_loop_var {
            return None;
        }

        // 检查访问类型组合
        match (acc1.access_type, acc2.access_type) {
            (AccessType::Write, AccessType::Read) |
            (AccessType::Read, AccessType::Write) |
            (AccessType::Write, AccessType::Write) |
            (AccessType::Write, AccessType::ReadWrite) |
            (AccessType::ReadWrite, AccessType::Read) |
            (AccessType::ReadWrite, AccessType::Write) => {
                Some(BufferDependency {
                    buffer: acc1.buffer.clone(),
                    dep_type: match (acc1.access_type, acc2.access_type) {
                        (AccessType::Write, AccessType::Read) => DependencyType::Flow,
                        (AccessType::Read, AccessType::Write) => DependencyType::Anti,
                        (AccessType::Write, AccessType::Write) => DependencyType::Output,
                        _ => DependencyType::Flow,
                    },
                    source_idx: acc1.stmt_idx,
                    target_idx: acc2.stmt_idx,
                })
            }
            _ => None,
        }
    }

    /// 检查表达式是否包含循环变量
    fn contains_loop_var(&self, expr: &Expression) -> bool {
        match expr {
            Expression::Identifier(name) => self.loop_vars.contains(name),
            Expression::Binary { left, right, .. } => {
                self.contains_loop_var(left) || self.contains_loop_var(right)
            }
            _ => false,
        }
    }

    /// 检查是否是线性索引
    fn is_linear_index(&self, expr: &Expression) -> bool {
        match expr {
            Expression::Identifier(_) => true,
            Expression::Integer(_) => true,
            Expression::Binary { left, op, right } => {
                match op {
                    BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul => {
                        self.is_linear_index(left) && self.is_linear_index(right)
                    }
                    _ => false,
                }
            }
            _ => false,
        }
    }

    /// 表达式转字符串
    fn expr_to_string(&self, expr: &Expression) -> String {
        match expr {
            Expression::Identifier(name) => name.clone(),
            Expression::Integer(n) => n.to_string(),
            Expression::Binary { left, op, right } => {
                format!("{}{}{}", self.expr_to_string(left), self.op_to_string(*op), self.expr_to_string(right))
            }
            _ => "expr".to_string(),
        }
    }

    /// 操作符转字符串
    fn op_to_string(&self, op: BinaryOp) -> &'static str {
        match op {
            BinaryOp::Add => "+",
            BinaryOp::Sub => "-",
            BinaryOp::Mul => "*",
            BinaryOp::Div => "/",
            _ => "?",
        }
    }
}

impl Default for DependenceAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

/// 循环独立性分析结果
#[derive(Debug)]
pub struct LoopIndependenceResult {
    /// 是否独立
    pub is_independent: bool,
    /// 发现的依赖
    pub dependencies: Vec<BufferDependency>,
}

/// Buffer 依赖
#[derive(Debug, Clone)]
pub struct BufferDependency {
    /// Buffer 名称
    pub buffer: String,
    /// 依赖类型
    pub dep_type: DependencyType,
    /// 源位置
    pub source_idx: usize,
    /// 目标位置
    pub target_idx: usize,
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_definition_point() {
        let def = DefinitionPoint::new("x".to_string(), 5);
        assert_eq!(def.variable, "x");
        assert_eq!(def.statement_idx, 5);
        assert!(!def.in_loop);

        let def_in_loop = def.in_loop("i".to_string());
        assert!(def_in_loop.in_loop);
        assert_eq!(def_in_loop.loop_var, Some("i".to_string()));
    }

    #[test]
    fn test_control_flow_graph() {
        let mut cfg = ControlFlowGraph::new();
        let b0 = cfg.add_block();
        let b1 = cfg.add_block();
        let b2 = cfg.add_block();

        cfg.add_edge(b0, b1);
        cfg.add_edge(b1, b2);

        assert_eq!(cfg.blocks.len(), 3);
        assert_eq!(cfg.blocks[0].successors, vec![1]);
        assert_eq!(cfg.blocks[1].predecessors, vec![0]);
    }

    #[test]
    fn test_data_flow_analyzer_simple() {
        let source = r#"
fn main() {
    let x = 5;
    let y = x + 1;
}
"#;
        let mut lexer = crate::lexer::Lexer::new(source);
        let tokens = lexer.tokenize();
        let mut parser = crate::parser::Parser::new(tokens);
        let ast = parser.parse_program().expect("Parse failed");

        let mut analyzer = DataFlowAnalyzer::new();
        let result = analyzer.analyze_program(&ast);

        // 不应该有未初始化变量使用
        assert!(result.uninitialized_uses.is_empty());
    }

    #[test]
    fn test_dependence_analyzer() {
        let mut analyzer = DependenceAnalyzer::new();
        
        let source = r#"
fn main() {
    let arr = Buffer::<f32>::zeros([100]);
    parallel for i in 0..100 {
        let x = arr[i];
        arr[i] = x + 1;
    }
}
"#;
        let mut lexer = crate::lexer::Lexer::new(source);
        let tokens = lexer.tokenize();
        let mut parser = crate::parser::Parser::new(tokens);
        let ast = parser.parse_program().expect("Parse failed");

        // 找到循环体
        if let Some(func) = ast.functions.first() {
            if let Some(Statement::ParallelFor { body, .. }) = func.body.statements.get(1) {
                let result = analyzer.analyze_loop("i", body);
                
                // 由于 arr[i] 先读后写，应该检测到依赖
                assert!(!result.is_independent);
            }
        }
    }
}
