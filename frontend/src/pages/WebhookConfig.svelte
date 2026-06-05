<script>
  import { onMount } from 'svelte'
  import Card from '../components/Card.svelte'
  import {
    getWebhooks,
    createWebhook,
    updateWebhook,
    deleteWebhook,
    testWebhook,
  } from '../api.js'

  let webhooks = []
  let loading = true
  let showModal = false
  let editingWebhook = null
  let formData = {
    name: '',
    url: '',
    threshold_score_drop: 20,
    cooldown_minutes: 60,
    is_active: true,
  }
  let testingId = null
  let testResult = null

  onMount(async () => {
    await loadWebhooks()
  })

  async function loadWebhooks() {
    loading = true
    try {
      const res = await getWebhooks()
      webhooks = res.data
    } catch (e) {
      console.error('Failed to load webhooks:', e)
    } finally {
      loading = false
    }
  }

  function openCreateModal() {
    editingWebhook = null
    formData = {
      name: '',
      url: '',
      threshold_score_drop: 20,
      cooldown_minutes: 60,
      is_active: true,
    }
    showModal = true
    testResult = null
  }

  function openEditModal(webhook) {
    editingWebhook = webhook
    formData = {
      name: webhook.name,
      url: webhook.url,
      threshold_score_drop: webhook.threshold_score_drop,
      cooldown_minutes: webhook.cooldown_minutes,
      is_active: webhook.is_active,
    }
    showModal = true
    testResult = null
  }

  function closeModal() {
    showModal = false
    editingWebhook = null
    testResult = null
  }

  async function handleSubmit() {
    if (!formData.name || !formData.url) {
      alert('请填写名称和回调URL')
      return
    }

    try {
      if (editingWebhook) {
        await updateWebhook(editingWebhook.id, formData)
        alert('Webhook配置已更新')
      } else {
        await createWebhook(formData)
        alert('Webhook配置已创建')
      }
      closeModal()
      await loadWebhooks()
    } catch (e) {
      alert('保存失败')
    }
  }

  async function handleDelete(id) {
    if (!confirm('确定要删除这个Webhook配置吗？')) {
      return
    }
    try {
      await deleteWebhook(id)
      await loadWebhooks()
    } catch (e) {
      alert('删除失败')
    }
  }

  async function handleTest(id) {
    testingId = id
    testResult = null
    try {
      const res = await testWebhook(id)
      testResult = res.data
      if (res.data.success) {
        alert('测试发送成功！状态码: ' + res.data.response_status)
      } else {
        alert('测试发送失败！状态码: ' + res.data.response_status)
      }
    } catch (e) {
      alert('测试发送失败: ' + e.message)
    } finally {
      testingId = null
    }
  }

  function formatDate(dateStr) {
    if (!dateStr) return '-'
    return new Date(dateStr).toLocaleString('zh-CN')
  }
</script>

<div class="webhook-page">
  <div class="page-header">
    <div>
      <h1>Webhook 配置</h1>
      <p class="subtitle">管理健康评分变化的通知回调</p>
    </div>
    <button class="btn-primary" on:click={openCreateModal}>
      ➕ 添加 Webhook
    </button>
  </div>

  {#if loading}
    <div class="loading">加载中...</div>
  {:else}
    <Card>
      {#if webhooks.length === 0}
        <div class="empty">
          <p>暂无 Webhook 配置</p>
          <p class="empty-tip">点击右上角"添加 Webhook"创建第一个通知配置</p>
        </div>
      {:else}
        <div class="webhook-list">
          {#each webhooks as webhook}
            <div class="webhook-card">
              <div class="webhook-header">
                <div class="webhook-title">
                  <span class="webhook-name">{webhook.name}</span>
                  <span class="status-badge" class:active={webhook.is_active}>
                    {webhook.is_active ? '启用' : '禁用'}
                  </span>
                </div>
                <div class="webhook-actions">
                  <button class="btn-test" disabled={testingId === webhook.id} on:click={() => handleTest(webhook.id)}>
                    {testingId === webhook.id ? '测试中...' : '测试'}
                  </button>
                  <button class="btn-edit" on:click={() => openEditModal(webhook)}>编辑</button>
                  <button class="btn-delete" on:click={() => handleDelete(webhook.id)}>删除</button>
                </div>
              </div>
              <div class="webhook-details">
                <div class="detail-row">
                  <span class="label">回调URL:</span>
                  <span class="value url-value">{webhook.url}</span>
                </div>
                <div class="detail-row">
                  <span class="label">触发阈值:</span>
                  <span class="value">分数下降 {webhook.threshold_score_drop} 分</span>
                </div>
                <div class="detail-row">
                  <span class="label">冷却期:</span>
                  <span class="value">{webhook.cooldown_minutes} 分钟</span>
                </div>
                <div class="detail-row">
                  <span class="label">上次触发:</span>
                  <span class="value">{formatDate(webhook.last_triggered_at)}</span>
                </div>
              </div>
            </div>
          {/each}
        </div>
      {/if}
    </Card>
  {/if}

  {#if showModal}
    <div class="modal-overlay" on:click={closeModal}>
      <div class="modal" on:click|stopPropagation>
        <div class="modal-header">
          <h3>{editingWebhook ? '编辑 Webhook' : '添加 Webhook'}</h3>
          <button class="close-btn" on:click={closeModal}>&times;</button>
        </div>
        <div class="modal-body">
          <div class="form-group">
            <label>名称 *</label>
            <input
              type="text"
              bind:value={formData.name}
              placeholder="例如：Slack通知"
            />
          </div>
          <div class="form-group">
            <label>回调 URL *</label>
            <input
              type="url"
              bind:value={formData.url}
              placeholder="https://..."
            />
          </div>
          <div class="form-row">
            <div class="form-group">
              <label>触发阈值 (分数下降)</label>
              <input
                type="number"
                min="1"
                max="100"
                bind:value={formData.threshold_score_drop}
              />
            </div>
            <div class="form-group">
              <label>冷却期 (分钟)</label>
              <input
                type="number"
                min="1"
                bind:value={formData.cooldown_minutes}
              />
            </div>
          </div>
          <div class="form-group checkbox-group">
            <input
              type="checkbox"
              bind:checked={formData.is_active}
              id="is_active"
            />
            <label for="is_active">启用此 Webhook</label>
          </div>
          <div class="payload-info">
            <p class="info-title">回调请求体包含:</p>
            <ul>
              <li><code>service_name</code> - 服务名称</li>
              <li><code>current_score</code> - 当前分数</li>
              <li><code>previous_score</code> - 上次分数</li>
              <li><code>score_drop</code> - 下降幅度</li>
              <li><code>triggered_at</code> - 触发时间</li>
              <li><code>dimension_changes</code> - 各维度分数变化</li>
            </ul>
          </div>
        </div>
        <div class="modal-footer">
          <button class="btn-secondary" on:click={closeModal}>取消</button>
          <button class="btn-primary" on:click={handleSubmit}>
            {editingWebhook ? '保存修改' : '创建'}
          </button>
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  .webhook-page {
    display: flex;
    flex-direction: column;
    gap: 20px;
  }

  .page-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
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

  .btn-primary {
    padding: 10px 20px;
    background: #3b82f6;
    border: none;
    border-radius: 8px;
    color: white;
    font-size: 14px;
    font-weight: 500;
    cursor: pointer;
    transition: background 0.2s;
  }

  .btn-primary:hover {
    background: #2563eb;
  }

  .btn-secondary {
    padding: 10px 20px;
    background: #334155;
    border: none;
    border-radius: 8px;
    color: #e2e8f0;
    font-size: 14px;
    font-weight: 500;
    cursor: pointer;
    transition: background 0.2s;
  }

  .btn-secondary:hover {
    background: #475569;
  }

  .loading, .empty {
    text-align: center;
    padding: 48px;
    color: #94a3b8;
  }

  .empty-tip {
    font-size: 13px;
    margin-top: 8px;
    color: #64748b;
  }

  .webhook-list {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .webhook-card {
    background: #0f172a;
    border-radius: 12px;
    padding: 20px;
    border: 1px solid #334155;
  }

  .webhook-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 16px;
  }

  .webhook-title {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .webhook-name {
    font-size: 16px;
    font-weight: 600;
    color: #f1f5f9;
  }

  .status-badge {
    padding: 4px 10px;
    border-radius: 20px;
    font-size: 12px;
    font-weight: 500;
    background: rgba(148, 163, 184, 0.15);
    color: #94a3b8;
  }

  .status-badge.active {
    background: rgba(34, 197, 94, 0.15);
    color: #22c55e;
  }

  .webhook-actions {
    display: flex;
    gap: 8px;
  }

  .btn-test {
    padding: 6px 14px;
    background: #8b5cf6;
    border: none;
    border-radius: 6px;
    color: white;
    font-size: 13px;
    cursor: pointer;
    transition: background 0.2s;
  }

  .btn-test:hover:not(:disabled) {
    background: #7c3aed;
  }

  .btn-test:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .btn-edit {
    padding: 6px 14px;
    background: #334155;
    border: none;
    border-radius: 6px;
    color: #e2e8f0;
    font-size: 13px;
    cursor: pointer;
    transition: background 0.2s;
  }

  .btn-edit:hover {
    background: #475569;
  }

  .btn-delete {
    padding: 6px 14px;
    background: rgba(239, 68, 68, 0.1);
    border: none;
    border-radius: 6px;
    color: #ef4444;
    font-size: 13px;
    cursor: pointer;
    transition: background 0.2s;
  }

  .btn-delete:hover {
    background: rgba(239, 68, 68, 0.2);
  }

  .webhook-details {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 12px;
  }

  .detail-row {
    display: flex;
    gap: 8px;
    font-size: 13px;
  }

  .detail-row .label {
    color: #94a3b8;
    min-width: 80px;
  }

  .detail-row .value {
    color: #e2e8f0;
  }

  .url-value {
    font-family: monospace;
    word-break: break-all;
  }

  .modal-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.6);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
    padding: 20px;
  }

  .modal {
    background: #1e293b;
    border-radius: 12px;
    width: 100%;
    max-width: 550px;
    max-height: 90vh;
    overflow-y: auto;
  }

  .modal-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 20px 24px;
    border-bottom: 1px solid #334155;
  }

  .modal-header h3 {
    margin: 0;
    font-size: 18px;
    color: #f1f5f9;
  }

  .close-btn {
    background: none;
    border: none;
    font-size: 24px;
    color: #94a3b8;
    cursor: pointer;
    padding: 0;
    line-height: 1;
  }

  .close-btn:hover {
    color: #e2e8f0;
  }

  .modal-body {
    padding: 24px;
  }

  .form-group {
    margin-bottom: 16px;
  }

  .form-group label {
    display: block;
    font-size: 13px;
    color: #94a3b8;
    margin-bottom: 6px;
  }

  .form-group input[type="text"],
  .form-group input[type="url"],
  .form-group input[type="number"] {
    width: 100%;
    padding: 10px 12px;
    background: #0f172a;
    border: 1px solid #334155;
    border-radius: 8px;
    color: #e2e8f0;
    font-size: 14px;
    box-sizing: border-box;
  }

  .form-group input:focus {
    outline: none;
    border-color: #3b82f6;
  }

  .form-row {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 16px;
  }

  .checkbox-group {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .checkbox-group label {
    margin: 0;
    color: #e2e8f0;
    cursor: pointer;
  }

  .checkbox-group input[type="checkbox"] {
    width: 16px;
    height: 16px;
    cursor: pointer;
  }

  .payload-info {
    margin-top: 20px;
    padding: 16px;
    background: #0f172a;
    border-radius: 8px;
  }

  .info-title {
    font-size: 13px;
    font-weight: 600;
    color: #e2e8f0;
    margin: 0 0 8px 0;
  }

  .payload-info ul {
    margin: 0;
    padding-left: 20px;
    font-size: 12px;
    color: #94a3b8;
  }

  .payload-info li {
    margin-bottom: 4px;
  }

  .payload-info code {
    background: #1e293b;
    padding: 2px 6px;
    border-radius: 4px;
    color: #3b82f6;
  }

  .modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: 12px;
    padding: 16px 24px;
    border-top: 1px solid #334155;
  }
</style>
