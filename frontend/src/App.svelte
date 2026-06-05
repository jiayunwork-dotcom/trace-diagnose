<script>
  import { Router, Link, Route } from 'svelte-routing'
  import Navbar from './components/Navbar.svelte'
  import Dashboard from './pages/Dashboard.svelte'
  import Topology from './pages/Topology.svelte'
  import Traces from './pages/Traces.svelte'
  import TraceDetail from './pages/TraceDetail.svelte'
  import Analysis from './pages/Analysis.svelte'
  import SLO from './pages/SLO.svelte'
  import Import from './pages/Import.svelte'
</script>

<Router>
  <div class="app-layout">
    <Navbar />
    <main class="main-content">
      <Route path="/" component={Dashboard} />
      <Route path="/topology" component={Topology} />
      <Route path="/traces" component={Traces} />
      <Route path="/traces/:id" component={TraceDetail} />
      <Route path="/analysis" component={Analysis} />
      <Route path="/slo" component={SLO} />
      <Route path="/import" component={Import} />
    </main>
  </div>
</Router>

<style>
  .app-layout {
    min-height: 100vh;
    display: flex;
    flex-direction: column;
  }
  .main-content {
    flex: 1;
    padding: 24px;
    max-width: 1600px;
    width: 100%;
    margin: 0 auto;
  }
</style>
