<script>
  import { onMount } from 'svelte'
  import { navigate } from 'svelte-routing'
  import Card from '../components/Card.svelte'
  import {
    getAlertRules, createAlertRule, updateAlertRule, deleteAlertRule, getRuleEvents,
    getAlertEvents, acknowledgeAlert, evaluateAlerts, getServices
  } from '../api'

  let activeTab = 'rules'
  let rules = []
  let services = []
  let events = []
  let totalEvents = 0
  let page = 1
  let pageSize = 20
  let statusFilter = ''

  let showRuleModal = false
  let editingRule = null
  let showDetailModal = false
  let selectedEvent = null

  let ruleForm = {
    name: '',
    description: '',
    service_name: '',
    operation_name: '',
    metric_type: 'latency_p95',
    threshold: 1000,
    comparison_operator: '>',
    window_minutes: 5,
    consecutive_windows: 1,
    severity: 'warning',
    silence_minutes: 30,
    is_active: true
  }

  const metricTypes = [
    { value: 'latency_p95', label: 'P95延迟(ms)' },
    { value: 'latency_p50', label: 'P50延迟(ms)' },
    { value: 'error_rate', label: '错误率(%)' },
    { value: 'qps', label: 'QPS' }
  ]

  const operators = [
    { value: '>', label: '大于' },
    { value: '>=', label: '大于等于' },
    { value: '<', label: '小于' },
    { value: '<=', label: '小于等于' }
  ]

  const severities = [
    { value: 'critical', label: '严重' },
    { value: 'warning', label: '警告' },
    { value: 'info', label: '信息' }
  ]

  const statusOptions = [
    { value: '', label: '全部状态' },
    { value: 'firing', label: '触发中' },
    { value: 'resolved', label: '已恢复' },
    { value: 'acknowledged', label: '已确认' }
  ]

  onMount(async () => {
    await loadServices()
    await loadRules()
    await loadEvents()
  })

  async function loadServices() {
    try {
      const res = await getServices()
      services = res.data
    } catch (e) {
      console.error('Failed to load services:', e)
    }
  }

  async function loadRules() {
    try {
      const res = await getAlertRules()
      rules = res.data
    } catch (e) {
      console.error('Failed to load rules:', e)
    }
  }

  async function loadEvents() {
    try {
      const params = { page, page_size: pageSize }
      if (statusFilter) {
        params.status = statusFilter
      }
      const res = await getAlertEvents(params)
      events = res.data.data
      totalEvents = res.data.total
    } catch (e) {
      console.error('Failed to load events:', e)
    }
  }

  function openCreateRule() {
    editingRule = null
    ruleForm = {
      name: '',
      description: '',
      service_name: '',
      operation_name: '',
      metric_type: 'latency_p95',
      threshold: 1000,
      comparison_operator: '>',
      window_minutes: 5,
      consecutive_windows: 1,
      severity: 'warning',
      silence_minutes: 30,
      is_active: true
    }
    showRuleModal = true
  }

  function openEditRule(rule) {
    editingRule = rule
    ruleForm = { ...rule }
    showRuleModal = true
  }

  async function saveRule() {
    if (!ruleForm.name) {
      alert('请输入规则名称')
      return
    }
    try {
      if (editingRule) {
        await updateAlertRule(editingRule.id, ruleForm)
      } else {
        await createAlertRule(ruleForm)
      }
      showRuleModal = false
      await loadRules()
    } catch (e) {
      console.error('Failed to save rule:', e)
      alert('保存失败，请重试')
    }
  }

  async function removeRule(id) {
    if (confirm('确定要删除这条告警规则吗？')) {
      try {
        await deleteAlertRule(id)
        await loadRules()
      } catch (e) {
        console.error('Failed to delete rule:', e)
      }
    }
  }

  async function toggleRule(rule) {
    try {
      await updateAlertRule(rule.id, { is_active: !rule.is_active })
      await loadRules()
    } catch (e) {
      console.error('Failed to toggle rule:', e)
    }
  }

  function viewEventDetail(event) {
    selectedEvent = event
    showDetailModal = true
  }

  async function acknowledgeEvent(event) {
    if (confirm('确定要确认这条告警吗？')) {
      try {
        await acknowledgeAlert(event.id, { acknowledged_by: 'user' })
        await loadEvents()
        if (selectedEvent && selectedEvent.id === event.id) {
          selectedEvent.status = 'acknowledged'
        }
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
    if (page < Math.ceil(totalEvents / pageSize)) {
      page++
      loadEvents()
    }
  }

  function formatDate(dateStr) {
    return new Date(dateStr).toLocaleString('zh-CN')
  }

  function getMetricLabel(type) {
    return metricTypes.find(m => m.value === type)?.label || type
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

  function getSeverityBadge(severity) {
    const colors = {
      critical: 'bg-red-100 text-red-800',
      warning: 'bg-yellow-100 text-yellow-800',
      info: 'bg-blue-100 text-blue-800'
    }
    return colors[severity] || 'bg-gray-100 text-gray-800'
  }
</script>

<div class="alert-center-page">
  <div class="page-header">
    <h1>告警中心</h1>
  </div>

  <div class="tabs">
    <button class="tab {activeTab === 'rules' ? 'active' : ''}" on:click={() => activeTab = 'rules'}>
      告警规则
    </button>
    <button class="tab {activeTab === 'events' ? 'active' : ''}" on:click={() => activeTab = 'events'}>
      告警事件
    </button>
  </div>

  {#if activeTab === 'rules'}
    <div class="tab-content">
      <div class="tab-content-header">
        <h2>告警规则管理</h2>
        <button class="btn btn-primary" on:click={openCreateRule}>
          + 新建规则
        </button>
      </div>

      <Card>
        <div class="table-container">
          <table class="data-table">
            <thead>
              <tr>
                <th>规则名称</th>
                <th>服务</th>
                <th>指标</th>
                <th>阈值</th>
                <th>严重程度</th>
                <th>状态</th>
                <th>最后触发</th>
                <th>操作</th>
              </tr>
            </thead>
            <tbody>
              {#each rules as rule (rule.id)}
                <tr>
                  <td>
                    <div class="rule-name">{rule.name}</div>
                    {#if rule.description}
                      <div class="rule-desc">{rule.description}</div>
                    {/if}
                  </td>
                  <td>{rule.service_name || '全部服务'}</td>
                  <td>{getMetricLabel(rule.metric_type)}</td>
                  <td>{rule.comparison_operator} {rule.threshold}</td>
                  <td>
                    <span class="badge {getSeverityBadge(rule.severity)}">
                      {rule.severity}
                    </span>
                  </td>
                  <td>
                    <label class="toggle">
                      <input type="checkbox" checked={rule.is_active} on:change={() => toggleRule(rule)} />
                      <span class="toggle-slider"></span>
                    </label>
                  </td>
                  <td>{rule.last_triggered_at ? formatDate(rule.last_triggered_at) : '-'}</td>
                  <td>
                    <div class="action-buttons">
                      <button class="btn btn-sm btn-secondary" on:click={() => openEditRule(rule)}>
                        编辑
                      </button>
                      <button class="btn btn-sm btn-danger" on:click={() => removeRule(rule.id)}>
                        删除
                      </button>
                    </div>
                  </td>
                </tr>
              {/each}
              {#if rules.length === 0}
                <tr>
                  <td colspan="8" class="empty-state">暂无告警规则</td>
                </tr>
              {/if}
            </tbody>
          </table>
        </div>
      </Card>
    </div>
  {/if}

  {#if activeTab === 'events'}
    <div class="tab-content">
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
          共 {totalEvents} 条记录
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
                      <button class="btn btn-sm btn-secondary" on:click={() => viewEventDetail(event)}>
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

        {#if totalEvents > pageSize}
          <div class="pagination">
            <button class="btn btn-sm btn-secondary" on:click={prevPage} disabled={page === 1}>
              上一页
            </button>
            <span class="page-info">第 {page} / {Math.ceil(totalEvents / pageSize)} 页</span>
            <button class="btn btn-sm btn-secondary" on:click={nextPage} disabled={page >= Math.ceil(totalEvents / pageSize)}>
              下一页
            </button>
          </div>
        {/if}
      </Card>
    </div>
  {/if}

  {#if showRuleModal}
    <div class="modal-overlay" on:click={() => showRuleModal = false}>
      <div class="modal" on:click|stopPropagation>
        <div class="modal-header">
          <h3>{editingRule ? '编辑告警规则' : '新建告警规则'}</h3>
          <button class="modal-close" on:click={() => showRuleModal = false}>&times;</button>
        </div>
        <div class="modal-body">
          <div class="form-grid">
            <div class="form-group">
              <label>规则名称 *</label>
              <input type="text" bind:value={ruleForm.name} placeholder="输入规则名称" />
            </div>
            <div class="form-group">
              <label>严重程度</label>
              <select bind:value={ruleForm.severity}>
                {#each severities as s}
                  <option value={s.value}>{s.label}</option>
                {/each}
              </select>
            </div>
            <div class="form-group form-full">
              <label>描述</label>
              <textarea bind:value={ruleForm.description} placeholder="规则描述"></textarea>
            </div>
            <div class="form-group">
              <label>服务</label>
              <select bind:value={ruleForm.service_name}>
                <option value="">全部服务</option>
                {#each services as svc}
                  <option value={svc.service_name}>{svc.service_name}</option>
                {/each}
              </select>
            </div>
            <div class="form-group">
              <label>操作名</label>
              <input type="text" bind:value={ruleForm.operation_name} placeholder="留空表示全部操作" />
            </div>
            <div class="form-group">
              <label>指标类型 *</label>
              <select bind:value={ruleForm.metric_type}>
                {#each metricTypes as m}
                  <option value={m.value}>{m.label}</option>
                {/each}
              </select>
            </div>
            <div class="form-group">
              <label>比较符</label>
              <select bind:value={ruleForm.comparison_operator}>
                {#each operators as op}
                  <option value={op.value}>{op.label}</option>
                {/each}
              </select>
            </div>
            <div class="form-group">
              <label>阈值 *</label>
              <input type="number" bind:value={ruleForm.threshold} />
            </div>
            <div class="form-group">
              <label>时间窗口(分钟)</label>
              <input type="number" bind:value={ruleForm.window_minutes} min="1" />
            </div>
            <div class="form-group">
              <label>连续窗口数</label>
              <input type="number" bind:value={ruleForm.consecutive_windows} min="1" />
            </div>
            <div class="form-group">
              <label>静默期(分钟)</label>
              <input type="number" bind:value={ruleForm.silence_minutes} min="0" />
            </div>
          </div>
        </div>
        <div class="modal-footer">
          <button class="btn btn-secondary" on:click={() => showRuleModal = false}>取消</button>
          <button class="btn btn-primary" on:click={saveRule}>保存</button>
        </div>
      </div>
    </div>
  {/if}

  {#if showDetailModal && selectedEvent}
    <div class="modal-overlay" on:click={() => showDetailModal = false}>
      <div class="modal modal-lg" on:click|stopPropagation>
        <div class="modal-header">
          <h3>告警详情</h3>
          <button class="modal-close" on:click={() => showDetailModal = false}>&times;</button>
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
          <button class="btn btn-secondary" on:click={() => showDetailModal = false}>
            关闭
          </button>
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  .alert-center-page {
    display: flex;
    flex-direction: column;
    gap: 20px;
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
  }

  .tab {
    padding: 12px 24px;
    background: none;
    border: none;
    border-bottom: 2px solid transparent;
    color: #64748b;
    font-weight: 500;
    font-size: 14px;
    cursor: pointer;
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

  .tab-content {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .tab-content-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
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

  .rule-name, .event-name {
    font-weight: 500;
    color: #1e293b;
  }

  .rule-desc {
    font-size: 12px;
    color: #94a3b8;
    margin-top: 4px;
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

  .btn-danger {
    background: #ef4444;
    color: white;
  }

  .btn-danger:hover {
    background: #dc2626;
  }

  .btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .toggle {
    position: relative;
    display: inline-block;
    width: 44px;
    height: 24px;
  }

  .toggle input {
    opacity: 0;
    width: 0;
    height: 0;
  }

  .toggle-slider {
    position: absolute;
    cursor: pointer;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background-color: #cbd5e1;
    transition: .3s;
    border-radius: 24px;
  }

  .toggle-slider:before {
    position: absolute;
    content: "";
    height: 18px;
    width: 18px;
    left: 3px;
    bottom: 3px;
    background-color: white;
    transition: .3s;
    border-radius: 50%;
  }

  input:checked + .toggle-slider {
    background-color: #3b82f6;
  }

  input:checked + .toggle-slider:before {
    transform: translateX(20px);
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

  .form-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 16px;
  }

  .form-full {
    grid-column: 1 / -1;
  }

  .form-group {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .form-group label {
    font-size: 13px;
    font-weight: 500;
    color: #475569;
  }

  .form-group input,
  .form-group select,
  .form-group textarea {
    padding: 8px 12px;
    border: 1px solid #e2e8f0;
    border-radius: 6px;
    font-size: 14px;
  }

  .form-group textarea {
    resize: vertical;
    min-height: 60px;
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
