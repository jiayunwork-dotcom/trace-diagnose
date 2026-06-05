<script>
  export let title = ''
  export let subtitle = ''
</script>

<div class="card">
  {#if title}
    <div class="card-header">
      <h3 class="card-title">{title}</h3>
      {#if subtitle}
        <span class="card-subtitle">{subtitle}</span>
      {/if}
    </div>
  {/if}
  <div class="card-body">
    <slot />
  </div>
</div>

<style>
  .card {
    background: #1e293b;
    border: 1px solid #334155;
    border-radius: 12px;
    overflow: hidden;
  }
  .card-header {
    padding: 16px 20px;
    border-bottom: 1px solid #334155;
  }
  .card-title {
    font-size: 16px;
    font-weight: 600;
    color: #f1f5f9;
    margin: 0 0 4px 0;
  }
  .card-subtitle {
    font-size: 13px;
    color: #94a3b8;
  }
  .card-body {
    padding: 20px;
  }
</style>
