<script>
  import { onMount } from 'svelte'
  import { getAlertRules, createAlertRule, updateAlertRule, deleteAlertRule, getRuleEvents, getServices } from '../api'
  import Card from '../components/Card.svelte'
  import { Link, location } from 'svelte-routing'

  let rules = []
  let services = []
  let showModal = false
  let showEventsModal = false
  let editingRule = null
  let ruleEvents = []
  let selectedRuleId = null

  let form = {
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

  onMount(async () => {
    await loadRules()
    await loadServices()
  })

  async function loadRules() {
    try {
      const res = await getAlertRules()
      rules = res.data
    } catch (e) {
      console.error('Failed to load rules:', e)
    }
  }

  async function loadServices() {
    try {
      const res = await getServices()
      services = res.data
    } catch (e) {
      console.error('Failed to load services:', e)
    }
  }

  function openCreateModal() {
    editingRule = null
    form = {
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
    showModal = true
  }

  function openEditModal(rule) {
    editingRule = rule
    form = { ...rule }
    showModal = true
  }

  async function saveRule() {
    try {
      if (editingRule) {
        await updateAlertRule(editingRule.id, form)
      } else {
        await createAlertRule(form)
      }
      showModal = false
      await loadRules()
    } catch (e) {
      console.error('Failed to save rule:', e)
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

  async function viewEvents(ruleId) {
    selectedRuleId = ruleId
    try {
      const res = await getRuleEvents(ruleId)
      ruleEvents = res.data
      showEventsModal = true
    } catch (e) {
      console.error('Failed to load events:', e)
    }
  }

  function formatDate(dateStr) {
    return new Date(dateStr).toLocaleString('zh-CN')
  }

  function getMetricLabel(type) {
    return metricTypes.find(m => m.value === type)?.label || type
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

<div class="alert-rules-page">
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
    <h2>告警规则管理</h2>
    <button class="btn btn-primary" on:click={openCreateModal}>
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
                  <button class="btn btn-sm btn-secondary" on:click={() => viewEvents(rule.id)}>
                    触发历史
                  </button>
                  <button class="btn btn-sm btn-secondary" on:click={() => openEditModal(rule)}>
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

  {#if showModal}
    <div class="modal-overlay" on:click={() => showModal = false}>
      <div class="modal" on:click|stopPropagation>
        <div class="modal-header">
          <h3>{editingRule ? '编辑告警规则' : '新建告警规则'}</h3>
          <button class="modal-close" on:click={() => showModal = false}>&times;</button>
        </div>
        <div class="modal-body">
          <div class="form-grid">
            <div class="form-group">
              <label>规则名称 *</label>
              <input type="text" bind:value={form.name} placeholder="输入规则名称" />
            </div>
            <div class="form-group">
              <label>严重程度</label>
              <select bind:value={form.severity}>
                {#each severities as s}
                  <option value={s.value}>{s.label}</option>
                {/each}
              </select>
            </div>
            <div class="form-group form-full">
              <label>描述</label>
              <textarea bind:value={form.description} placeholder="规则描述"></textarea>
            </div>
            <div class="form-group">
              <label>服务</label>
              <select bind:value={form.service_name}>
                <option value="">全部服务</option>
                {#each services as svc}
                  <option value={svc.service_name}>{svc.service_name}</option>
                {/each}
              </select>
            </div>
            <div class="form-group">
              <label>操作名</label>
              <input type="text" bind:value={form.operation_name} placeholder="留空表示全部操作" />
            </div>
            <div class="form-group">
              <label>指标类型 *</label>
              <select bind:value={form.metric_type}>
                {#each metricTypes as m}
                  <option value={m.value}>{m.label}</option>
                {/each}
              </select>
            </div>
            <div class="form-group">
              <label>比较符</label>
              <select bind:value={form.comparison_operator}>
                {#each operators as op}
                  <option value={op.value}>{op.label}</option>
                {/each}
              </select>
            </div>
            <div class="form-group">
              <label>阈值 *</label>
              <input type="number" bind:value={form.threshold} />
            </div>
            <div class="form-group">
              <label>时间窗口(分钟)</label>
              <input type="number" bind:value={form.window_minutes} min="1" />
            </div>
            <div class="form-group">
              <label>连续窗口数</label>
              <input type="number" bind:value={form.consecutive_windows} min="1" />
            </div>
            <div class="form-group">
              <label>静默期(分钟)</label>
              <input type="number" bind:value={form.silence_minutes} min="0" />
            </div>
          </div>
        </div>
        <div class="modal-footer">
          <button class="btn btn-secondary" on:click={() => showModal = false}>取消</button>
          <button class="btn btn-primary" on:click={saveRule}>保存</button>
        </div>
      </div>
    </div>
  {/if}

  {#if showEventsModal}
    <div class="modal-overlay" on:click={() => showEventsModal = false}>
      <div class="modal modal-lg" on:click|stopPropagation>
        <div class="modal-header">
          <h3>触发历史</h3>
          <button class="modal-close" on:click={() => showEventsModal = false}>&times;</button>
        </div>
        <div class="modal-body">
          <div class="table-container">
            <table class="data-table">
              <thead>
                <tr>
                  <th>触发时间</th>
                  <th>状态</th>
                  <th>指标值</th>
                  <th>阈值</th>
                </tr>
              </thead>
              <tbody>
                {#each ruleEvents as event (event.id)}
                  <tr>
                    <td>{formatDate(event.created_at)}</td>
                    <td>
                      <span class="badge {event.status === 'firing' ? 'bg-red-100 text-red-800' : event.status === 'resolved' ? 'bg-green-100 text-green-800' : 'bg-blue-100 text-blue-800'}">
                        {event.status}
                      </span>
                    </td>
                    <td>{event.metric_value.toFixed(2)}</td>
                    <td>{event.threshold}</td>
                  </tr>
                {/each}
                {#if ruleEvents.length === 0}
                  <tr>
                    <td colspan="4" class="empty-state">暂无触发记录</td>
                  </tr>
                {/if}
              </tbody>
            </table>
          </div>
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  .alert-rules-page {
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

  .rule-name {
    font-weight: 500;
    color: #1e293b;
  }

  .rule-desc {
    font-size: 12px;
    color: #94a3b8;
    margin-top: 4px;
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
    max-width: 800px;
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
</style>
