<script>
  import { onMount } from 'svelte'
  import { navigate } from 'svelte-routing'
  import Card from '../components/Card.svelte'
  import { getTraces, getServices } from '../api.js'

  let traces = []
  let services = []
  let total = 0
  let page = 1
  let pageSize = 20
  let loading = true
  let filters = {
    service: '',
    minDuration: '',
    hasErrors: '',
  }

  onMount(async () => {
    await loadServices()
    await loadTraces()
  })

  async function loadServices() {
    try {
      const res = await getServices()
      services = res.data
    } catch (e) {
      console.error('Failed to load services:', e)
    }
  }

  async function loadTraces() {
    loading = true
    try {
      const params = { page, page_size: pageSize }
      if (filters.service) params.service = filters.service
      if (filters.minDuration) params.min_duration = parseInt(filters.minDuration)
      if (filters.hasErrors) params.has_errors = filters.hasErrors === 'true'

      const res = await getTraces(params)
      traces = res.data?.data || []
      total = res.data?.total || 0
    } catch (e) {
      console.error('Failed to load traces:', e)
    } finally {
      loading = false
    }
  }

  function viewTrace(traceId) {
    navigate(`/traces/${traceId}`)
  }

  function handleSearch() {
    page = 1
    loadTraces()
  }

  function prevPage() {
    if (page > 1) {
      page--
      loadTraces()
    }
  }

  function nextPage() {
    if (page * pageSize < total) {
      page++
      loadTraces()
    }
  }

  $: totalPages = Math.max(1, Math.ceil(total / pageSize))
</script>

<div class="traces-page">
  <div class="page-header">
    <h1>Trace搜索</h1>
    <p class="subtitle">按条件搜索和筛选调用链路</p>
  </div>

  <Card title="筛选条件">
    <div class="filter-row">
      <div class="filter-item">
        <label>服务</label>
        <select bind:value={filters.service}>
          <option value="">全部服务</option>
          {#each services as svc}
            <option value={svc.service_name}>{svc.service_name}</option>
          {/each}
        </select>
      </div>
      <div class="filter-item">
        <label>最小延迟 (ms)</label>
        <input type="number" bind:value={filters.minDuration} placeholder="例如: 100" />
      </div>
      <div class="filter-item">
        <label>状态</label>
        <select bind:value={filters.hasErrors}>
          <option value="">全部</option>
          <option value="true">有错误</option>
          <option value="false">正常</option>
        </select>
      </div>
      <div class="filter-item">
        <label>&nbsp;</label>
        <button class="search-btn" on:click={handleSearch}>搜索</button>
      </div>
    </div>
  </Card>

  <Card title={`Trace列表 (共 ${total} 条)`}>
    {#if loading}
      <div class="loading">加载中...</div>
    {:else if traces.length === 0}
      <div class="empty">暂无Trace数据，请先导入数据</div>
    {:else}
      <table class="trace-table">
        <thead>
          <tr>
            <th>Trace ID</th>
            <th>开始时间</th>
            <th>Span数</th>
            <th>服务数</th>
            <th>持续时间</th>
            <th>状态</th>
            <th>操作</th>
          </tr>
        </thead>
        <tbody>
          {#each traces as trace}
            <tr>
              <td class="trace-id">{trace.trace_id.slice(0, 24)}...</td>
              <td>{new Date(trace.start_time).toLocaleString()}</td>
              <td>{trace.span_count}</td>
              <td>{trace.service_count}</td>
              <td>{trace.duration_ms}ms</td>
              <td>
                <span class={`status-badge ${trace.has_errors ? 'error' : 'ok'}`}>
                  {trace.has_errors ? '有错误' : '正常'}
                </span>
              </td>
              <td>
                <button class="view-btn" on:click={() => viewTrace(trace.trace_id)}>查看</button>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>

      <div class="pagination">
        <button class="page-btn" on:click={prevPage} disabled={page === 1}>上一页</button>
        <span class="page-info">第 {page} / {totalPages || 1} 页</span>
        <button class="page-btn" on:click={nextPage} disabled={page >= totalPages}>下一页</button>
      </div>
    {/if}
  </Card>
</div>

<style>
  .traces-page {
    display: flex;
    flex-direction: column;
    gap: 24px;
  }
  .page-header h1 {
    font-size: 28px;
    font-weight: 700;
    color: #f1f5f9;
    margin: 0 0 8px 0;
  }
  .subtitle {
    color: #94a3b8;
    font-size: 14px;
    margin: 0;
  }
  .filter-row {
    display: flex;
    gap: 16px;
    flex-wrap: wrap;
    align-items: flex-end;
  }
  .filter-item {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .filter-item label {
    font-size: 13px;
    color: #94a3b8;
    font-weight: 500;
  }
  .filter-item select, .filter-item input {
    padding: 8px 12px;
    background: #0f172a;
    border: 1px solid #334155;
    border-radius: 6px;
    color: #e2e8f0;
    font-size: 14px;
    min-width: 150px;
  }
  .search-btn {
    padding: 8px 20px;
    background: #3b82f6;
    border: none;
    border-radius: 6px;
    color: white;
    font-size: 14px;
    font-weight: 500;
    cursor: pointer;
  }
  .search-btn:hover {
    background: #2563eb;
  }
  .loading, .empty {
    text-align: center;
    padding: 48px;
    color: #94a3b8;
  }
  .trace-table {
    width: 100%;
    border-collapse: collapse;
  }
  .trace-table th, .trace-table td {
    padding: 14px 12px;
    text-align: left;
    border-bottom: 1px solid #334155;
  }
  .trace-table th {
    font-size: 12px;
    font-weight: 600;
    color: #94a3b8;
    text-transform: uppercase;
  }
  .trace-id {
    font-family: monospace;
    color: #3b82f6;
  }
  .status-badge {
    padding: 4px 8px;
    border-radius: 6px;
    font-size: 12px;
    font-weight: 500;
  }
  .status-badge.ok { background: rgba(34, 197, 94, 0.1); color: #22c55e; }
  .status-badge.error { background: rgba(239, 68, 68, 0.1); color: #ef4444; }
  .view-btn {
    padding: 6px 12px;
    background: #334155;
    border: none;
    border-radius: 6px;
    color: #e2e8f0;
    font-size: 13px;
    cursor: pointer;
  }
  .view-btn:hover {
    background: #475569;
  }
  .pagination {
    display: flex;
    justify-content: center;
    align-items: center;
    gap: 16px;
    margin-top: 20px;
  }
  .page-btn {
    padding: 8px 16px;
    background: #334155;
    border: none;
    border-radius: 6px;
    color: #e2e8f0;
    font-size: 14px;
    cursor: pointer;
  }
  .page-btn:hover:not(:disabled) {
    background: #475569;
  }
  .page-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .page-info {
    color: #94a3b8;
    font-size: 14px;
  }
</style>
