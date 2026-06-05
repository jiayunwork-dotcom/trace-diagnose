<script>
  import { onMount } from 'svelte'
  import { getAlertEvents, acknowledgeAlert, evaluateAlerts } from '../api'
  import Card from '../components/Card.svelte'
  import { Link, location, navigate } from 'svelte-routing'

  let events = []
  let total = 0
  let page = 1
  let pageSize = 20
  let statusFilter = ''
  let showDetail = false
  let selectedEvent = null

  const statusOptions = [
    { value: '', label: '全部状态' },
    { value: 'firing', label: '触发中' },
    { value: 'resolved', label: '已恢复' },
    { value: 'acknowledged', label: '已确认' }
  ]

  onMount(async () => {
    await loadEvents()
  })

  async function loadEvents() {
    try {
      const params = { page, page_size: pageSize }
      if (statusFilter) {
        params.status = statusFilter
      }
      const res = await getAlertEvents(params)
      events = res.data.data
      total = res.data.total
    } catch (e) {
      console.error('Failed to load events:', e)
    }
  }

  function viewDetail(event) {
    selectedEvent = event
    showDetail = true
  }

  async function acknowledgeEvent(event) {
    if (confirm('确定要确认这条告警吗？')) {
      try {
        await acknowledgeAlert(event.id, { acknowledged_by: 'user' })
        await loadEvents()
      } catch (e) {
        console.error('Failed to acknowledge:', e)
      }
    }
  }

  async function handleEvaluate() {
    try {
      await evaluateAlerts()
      await loadEvents()
    } catch (e) {
      console.error('Failed to evaluate:', e)
    }
  }

  function formatDate(dateStr) {
    return new Date(dateStr).toLocaleString('zh-CN')
  }

  function getStatusBadge(status) {
    const styles = {
      firing: 'bg-red-100 text-red-800',
      resolved: 'bg-green-100 text-green-800',
      acknowledged: 'bg-blue-100 text-blue-800'
    }
    return styles[status] || 'bg-gray-100 text-gray-800'
  }

  function getStatusLabel(status) {
    const labels = {
      firing: '触发中',
      resolved: '已恢复',
      acknowledged: '已确认'
    }
    return labels[status] || status
  }

  function getMetricLabel(type) {
    const labels = {
      latency_p95: 'P95延迟(ms)',
      latency_p50: 'P50延迟(ms)',
      error_rate: '错误率(%)',
      qps: 'QPS'
    }
    return labels[type] || type
  }

  function goToTrace(traceId) {
    navigate(`/traces/${traceId}`)
  }

  function prevPage() {
    if (page > 1) {
      page--
      loadEvents()
    }
  }

  function nextPage() {
    if (page < Math.ceil(total / pageSize)) {
      page++
      loadEvents()
    }
  }
</script>

<div class="alert-events-page">
  <div class="page-header">
    <h1>告警中心</h1>
  </div>

  <div class="tabs">
    <Link to="/alerts/rules" class="tab" class:active={$location.pathname === '/alerts/rules'}>
      告警规则
    </Link>
    <Link to="/alerts/events" class="tab" class:active={$location.pathname === '/alerts/events'}>
      告警事件
    </Link>
  </div>

  <div class="tab-content-header">
    <h2>告警事件</h2>
    <div class="header-actions">
      <button class="btn btn-secondary" on:click={handleEvaluate}>
        立即评估
      </button>
    </div>
  </div>

  <div class="filter-bar">
    <div class="filter-item">
      <label>状态筛选</label>
      <select bind:value={statusFilter} on:change={loadEvents}>
        {#each statusOptions as opt}
          <option value={opt.value}>{opt.label}</option>
        {/each}
      </select>
    </div>
    <div class="filter-stats">
      共 {total} 条记录
    </div>
  </div>

  <Card>
    <div class="table-container">
      <table class="data-table">
        <thead>
          <tr>
            <th>告警名称</th>
            <th>服务</th>
            <th>指标</th>
            <th>当前值</th>
            <th>阈值</th>
            <th>状态</th>
            <th>触发时间</th>
            <th>操作</th>
          </tr>
        </thead>
        <tbody>
          {#each events as event (event.id)}
            <tr>
              <td>
                <span class="event-name">{event.rule_name}</span>
              </td>
              <td>{event.service_name || '-'}</td>
              <td>{getMetricLabel(event.metric_type)}</td>
              <td class="metric-value">{event.metric_value.toFixed(2)}</td>
              <td>{event.threshold}</td>
              <td>
                <span class="badge {getStatusBadge(event.status)}">
                  {getStatusLabel(event.status)}
                </span>
              </td>
              <td>{formatDate(event.created_at)}</td>
              <td>
                <div class="action-buttons">
                  <button class="btn btn-sm btn-secondary" on:click={() => viewDetail(event)}>
                    详情
                  </button>
                  {#if event.status === 'firing'}
                    <button class="btn btn-sm btn-primary" on:click={() => acknowledgeEvent(event)}>
                      确认
                    </button>
                  {/if}
                </div>
              </td>
            </tr>
          {/each}
          {#if events.length === 0}
            <tr>
              <td colspan="8" class="empty-state">暂无告警事件</td>
            </tr>
          {/if}
        </tbody>
      </table>
    </div>

    {#if total > pageSize}
      <div class="pagination">
        <button class="btn btn-sm btn-secondary" on:click={prevPage} disabled={page === 1}>
          上一页
        </button>
        <span class="page-info">第 {page} / {Math.ceil(total / pageSize)} 页</span>
        <button class="btn btn-sm btn-secondary" on:click={nextPage} disabled={page >= Math.ceil(total / pageSize)}>
          下一页
        </button>
      </div>
    {/if}
  </Card>

  {#if showDetail && selectedEvent}
    <div class="modal-overlay" on:click={() => showDetail = false}>
      <div class="modal modal-lg" on:click|stopPropagation>
        <div class="modal-header">
          <h3>告警详情</h3>
          <button class="modal-close" on:click={() => showDetail = false}>&times;</button>
        </div>
        <div class="modal-body">
          <div class="detail-grid">
            <div class="detail-item">
              <label>告警名称</label>
              <div class="detail-value">{selectedEvent.rule_name}</div>
            </div>
            <div class="detail-item">
              <label>状态</label>
              <div class="detail-value">
                <span class="badge {getStatusBadge(selectedEvent.status)}">
                  {getStatusLabel(selectedEvent.status)}
                </span>
              </div>
            </div>
            <div class="detail-item">
              <label>服务</label>
              <div class="detail-value">{selectedEvent.service_name || '-'}</div>
            </div>
            <div class="detail-item">
              <label>操作</label>
              <div class="detail-value">{selectedEvent.operation_name || '-'}</div>
            </div>
            <div class="detail-item">
              <label>指标类型</label>
              <div class="detail-value">{getMetricLabel(selectedEvent.metric_type)}</div>
            </div>
            <div class="detail-item">
              <label>当前值</label>
              <div class="detail-value metric-value">{selectedEvent.metric_value.toFixed(2)}</div>
            </div>
            <div class="detail-item">
              <label>阈值</label>
              <div class="detail-value">{selectedEvent.threshold}</div>
            </div>
            <div class="detail-item">
              <label>触发时间</label>
              <div class="detail-value">{formatDate(selectedEvent.created_at)}</div>
            </div>
            {#if selectedEvent.acknowledged_at}
              <div class="detail-item">
                <label>确认时间</label>
                <div class="detail-value">{formatDate(selectedEvent.acknowledged_at)}</div>
              </div>
              <div class="detail-item">
                <label>确认人</label>
                <div class="detail-value">{selectedEvent.acknowledged_by || '-'}</div>
              </div>
            {/if}
            {#if selectedEvent.resolved_at}
              <div class="detail-item">
                <label>恢复时间</label>
                <div class="detail-value">{formatDate(selectedEvent.resolved_at)}</div>
              </div>
            {/if}
          </div>

          {#if selectedEvent.trace_ids && selectedEvent.trace_ids.length > 0}
            <div class="section">
              <h4>关联异常Trace</h4>
              <div class="trace-list">
                {#each selectedEvent.trace_ids as traceId}
                  <div class="trace-item" on:click={() => goToTrace(traceId)}>
                    <span class="trace-id">{traceId}</span>
                    <span class="trace-link">查看 →</span>
                  </div>
                {/each}
              </div>
            </div>
          {/if}
        </div>
        <div class="modal-footer">
          {#if selectedEvent.status === 'firing'}
            <button class="btn btn-primary" on:click={() => acknowledgeEvent(selectedEvent)}>
              确认告警
            </button>
          {/if}
          <button class="btn btn-secondary" on:click={() => showDetail = false}>
            关闭
          </button>
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  .alert-events-page {
    display: flex;
    flex-direction: column;
    gap: 20px;
  }

  .page-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .page-header h1 {
    margin: 0;
    font-size: 24px;
    color: #1e293b;
  }

  .tabs {
    display: flex;
    gap: 4px;
    border-bottom: 2px solid #e2e8f0;
    margin-bottom: 20px;
  }

  .tab {
    padding: 12px 24px;
    text-decoration: none;
    color: #64748b;
    font-weight: 500;
    font-size: 14px;
    border-bottom: 2px solid transparent;
    margin-bottom: -2px;
    transition: all 0.2s;
  }

  .tab:hover {
    color: #3b82f6;
  }

  .tab.active {
    color: #3b82f6;
    border-bottom-color: #3b82f6;
  }

  .tab-content-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 16px;
  }

  .tab-content-header h2 {
    margin: 0;
    font-size: 18px;
    color: #1e293b;
  }

  .header-actions {
    display: flex;
    gap: 12px;
  }

  .filter-bar {
    display: flex;
    align-items: center;
    gap: 20px;
    padding: 16px;
    background: #f8fafc;
    border-radius: 8px;
  }

  .filter-item {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .filter-item label {
    font-size: 14px;
    color: #64748b;
    font-weight: 500;
  }

  .filter-item select {
    padding: 6px 12px;
    border: 1px solid #e2e8f0;
    border-radius: 6px;
    background: white;
    font-size: 14px;
  }

  .filter-stats {
    margin-left: auto;
    color: #64748b;
    font-size: 14px;
  }

  .table-container {
    overflow-x: auto;
  }

  .data-table {
    width: 100%;
    border-collapse: collapse;
  }

  .data-table th {
    background: #f8fafc;
    padding: 12px 16px;
    text-align: left;
    font-weight: 600;
    font-size: 13px;
    color: #64748b;
    border-bottom: 2px solid #e2e8f0;
  }

  .data-table td {
    padding: 12px 16px;
    border-bottom: 1px solid #e2e8f0;
  }

  .event-name {
    font-weight: 500;
    color: #1e293b;
  }

  .metric-value {
    font-weight: 600;
    color: #ef4444;
  }

  .badge {
    display: inline-block;
    padding: 4px 8px;
    border-radius: 4px;
    font-size: 12px;
    font-weight: 500;
  }

  .action-buttons {
    display: flex;
    gap: 8px;
  }

  .btn {
    padding: 8px 16px;
    border: none;
    border-radius: 6px;
    cursor: pointer;
    font-size: 14px;
    font-weight: 500;
    transition: all 0.2s;
  }

  .btn-sm {
    padding: 4px 12px;
    font-size: 12px;
  }

  .btn-primary {
    background: #3b82f6;
    color: white;
  }

  .btn-primary:hover {
    background: #2563eb;
  }

  .btn-secondary {
    background: #e2e8f0;
    color: #475569;
  }

  .btn-secondary:hover {
    background: #cbd5e1;
  }

  .btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .empty-state {
    text-align: center;
    color: #94a3b8;
    padding: 40px;
  }

  .pagination {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 16px;
    padding: 16px;
    border-top: 1px solid #e2e8f0;
  }

  .page-info {
    color: #64748b;
    font-size: 14px;
  }

  .modal-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .modal {
    background: white;
    border-radius: 12px;
    width: 90%;
    max-width: 600px;
    max-height: 90vh;
    display: flex;
    flex-direction: column;
  }

  .modal-lg {
    max-width: 700px;
  }

  .modal-header {
    padding: 20px 24px;
    border-bottom: 1px solid #e2e8f0;
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .modal-header h3 {
    margin: 0;
    font-size: 18px;
  }

  .modal-close {
    background: none;
    border: none;
    font-size: 24px;
    cursor: pointer;
    color: #94a3b8;
  }

  .modal-body {
    padding: 24px;
    overflow-y: auto;
    flex: 1;
  }

  .modal-footer {
    padding: 16px 24px;
    border-top: 1px solid #e2e8f0;
    display: flex;
    justify-content: flex-end;
    gap: 12px;
  }

  .detail-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 16px;
  }

  .detail-item {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .detail-item label {
    font-size: 12px;
    color: #94a3b8;
    font-weight: 500;
  }

  .detail-value {
    font-size: 14px;
    color: #1e293b;
  }

  .section {
    margin-top: 24px;
    padding-top: 16px;
    border-top: 1px solid #e2e8f0;
  }

  .section h4 {
    margin: 0 0 12px 0;
    font-size: 14px;
    color: #475569;
  }

  .trace-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .trace-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 10px 12px;
    background: #f8fafc;
    border-radius: 6px;
    cursor: pointer;
    transition: background 0.2s;
  }

  .trace-item:hover {
    background: #e2e8f0;
  }

  .trace-id {
    font-family: monospace;
    font-size: 13px;
    color: #475569;
  }

  .trace-link {
    font-size: 12px;
    color: #3b82f6;
  }
</style>
