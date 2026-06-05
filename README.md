# Trace Diagnose - 微服务链路追踪分析与性能诊断平台

一个功能完整的微服务链路追踪数据分析与性能诊断平台，支持分布式追踪数据导入、服务调用拓扑重建、性能瓶颈定位和可视化展示。

## 功能特性

### 📊 数据导入模块
- **多格式支持**: OpenTelemetry JSON/Protobuf、Jaeger JSON、Zipkin v2 JSON
- **数据校验**: 缺失必要字段的Span标记为异常而非丢弃
- **导入方式**: 批量文件上传 + API推送
- **进度显示**: 大文件导入实时进度展示

### 🕸️ 调用链拓扑重建
- **Trace树构建**: 根据Span父子关系重建完整调用树
- **服务依赖图**: 服务节点 + 有向边（调用次数、平均延迟）
- **循环检测**: 自动检测并标记循环调用
- **增量更新**: 新数据自动合并到已有依赖图

### 📈 性能分析
- **延迟分位数**: P50、P95、P99计算
- **错误率统计**: 非200状态码比例
- **QPS计算**: 多时间窗口吞吐量
- **延迟直方图**: 识别双峰分布等异常模式
- **多维度下钻**: 按时间段、服务名、操作名筛选

### 🔍 异常链路检测
- **超时/错误标记**: 自动识别异常Span
- **异常传播路径**: 追踪上游调用者和下游依赖
- **根源定位**: 从错误点向上游回溯
- **严重程度排序**: 高频异常优先展示

### 🛤️ 关键路径分析
- **串行瓶颈识别**: 找出决定总延迟的Span链
- **贡献比例**: 每个Span对总延迟的贡献
- **并行优化建议**: 识别可并行化的串行Span

### 🔄 Trace对比
- **结构化差异对比**: 正常 vs 异常Trace
- **高亮显示**: 新增/删除/延迟变化的Span
- **性能回归定位**: 快速定位变更引入的问题

### 🎯 SLO监控
- **服务级别目标定义**: 延迟、错误率等
- **错误预算计算**: 剩余预算量
- **燃尽图**: 预算消耗趋势
- **预警机制**: 预算耗尽前提醒

### 🖥️ Web管理界面
- **Dashboard**: 全局健康概览、指标趋势、异常告警
- **服务拓扑图**: 力导向图布局，节点大小=QPS，颜色=错误率
- **Trace搜索**: 条件筛选、瀑布图展示
- **性能分析**: 延迟直方图、分位数趋势
- **SLO管理**: 目标配置、达标状态、燃尽图
- **数据导入**: 拖拽上传、历史记录

## 技术栈

### 后端
- **Rust 1.82+** - 高性能安全的系统编程语言
- **Axum** - 异步Web框架
- **SQLx** - 类型安全的数据库访问
- **Redis** - 热点数据缓存
- **PostgreSQL 16** - 数据持久化存储

### 前端
- **Svelte 4** - 编译式前端框架
- **D3.js** - 拓扑图可视化
- **Chart.js** - 图表展示
- **Axios** - HTTP客户端

### 容器化
- **rust:1.82-alpine** - 后端编译环境
- **debian:bookworm-slim** - 后端运行环境
- **nginx:alpine** - 前端托管
- **postgres:16-alpine** - 数据库
- **redis:7-alpine** - 缓存

## 快速开始

### 使用 Docker Compose (推荐)

```bash
# 克隆项目
cd trace-diagnose

# 启动所有服务
docker-compose up -d --build

# 访问前端界面
# http://localhost:3000

# 后端API
# http://localhost:8080/api/health
```

### 本地开发

#### 后端
```bash
cd backend

# 确保PostgreSQL和Redis已启动
# 设置环境变量
export DATABASE_URL=postgres://postgres:postgres@localhost:5432/trace_diagnose
export REDIS_URL=redis://localhost:6379

# 运行
cargo run
```

#### 前端
```bash
cd frontend

# 安装依赖
npm install

# 启动开发服务器
npm run dev
```

## API 接口

### Trace管理
- `GET /api/traces` - 分页查询Trace列表
- `GET /api/traces/:id` - 获取单条Trace详情
- `GET /api/traces/:id/spans` - 获取Trace的所有Span
- `GET /api/traces/:id/critical-path` - 获取关键路径
- `GET /api/traces/compare?baseline=xxx&comparison=yyy` - 对比两条Trace

### 服务管理
- `GET /api/services` - 获取所有服务列表
- `GET /api/services/:name` - 获取服务详情
- `GET /api/services/:name/metrics` - 获取服务指标

### 拓扑
- `GET /api/topology` - 获取服务拓扑图

### 分析
- `GET /api/analysis/latency-distribution` - 延迟分布
- `GET /api/analysis/anomalies` - 异常列表

### SLO管理
- `GET /api/slo` - 获取所有SLO
- `POST /api/slo` - 创建SLO
- `GET /api/slo/:id` - 获取SLO详情
- `PUT /api/slo/:id` - 更新SLO
- `DELETE /api/slo/:id` - 删除SLO
- `GET /api/slo/:id/status` - 获取SLO状态

### 数据导入
- `POST /api/import/upload` - 文件上传导入
- `POST /api/import/push?format=otel` - API推送导入
- `GET /api/import/jobs` - 导入任务列表
- `GET /api/import/jobs/:id` - 任务详情
- `GET /api/import/jobs/:id/progress` - 任务进度

## 数据导入示例

### 上传文件
```bash
curl -X POST http://localhost:8080/api/import/upload \
  -F "format=otel" \
  -F "file=@examples/sample-otel.json"
```

### API推送
```bash
curl -X POST http://localhost:8080/api/import/push?format=otel \
  -H "Content-Type: application/json" \
  -d @examples/sample-otel.json
```

## 项目结构

```
trace-diagnose/
├── backend/
│   ├── src/
│   │   ├── main.rs          # 应用入口
│   │   ├── config.rs        # 配置管理
│   │   ├── models.rs        # 数据模型
│   │   ├── db.rs            # 数据库连接
│   │   ├── cache.rs         # Redis缓存
│   │   ├── importer.rs      # 数据导入解析
│   │   ├── analysis.rs      # 分析计算引擎
│   │   └── handlers.rs      # API路由处理
│   ├── migrations/          # 数据库迁移
│   ├── Cargo.toml
│   └── Dockerfile
├── frontend/
│   ├── src/
│   │   ├── pages/           # 页面组件
│   │   ├── components/      # 通用组件
│   │   ├── api.js           # API客户端
│   │   ├── App.svelte
│   │   └── main.js
│   ├── nginx.conf
│   ├── package.json
│   └── Dockerfile
├── examples/                # 示例数据
├── docker-compose.yml
└── README.md
```

## License

MIT
