/* mesh-latency.js — Live mesh network metrics dashboard */
;(function() {
  'use strict';

  let _interval = null;
  let _prevNet = {};
  let _prevSync = {};

  function renderMeshLatency() {
    // No independent interval — called by refreshAll on global schedule
    updateAll();
  }

  function updateAll() {
    var container = document.getElementById('mesh-latency-container');
    if (!container) return;

    Promise.all([
      fetch('/api/mesh/traffic').then(function(r) { return r.json(); }).catch(function() { return null; }),
      fetch('/api/mesh').then(function(r) { return r.json(); }).catch(function() { return null; })
    ]).then(function(results) {
      var traffic = results[0];
      var meshData = results[1];
      var M = window.Maranello;
      if (!M || typeof M.speedometer !== 'function') return;

      var peers = [];
      if (meshData && Array.isArray(meshData.peers)) peers = meshData.peers;
      else if (Array.isArray(meshData)) peers = meshData;
      var localNode = (traffic && traffic.local_node) || 'mac-worker-2';

      // Merge traffic data
      var syncMap = {};
      if (traffic && traffic.sync_peers) {
        traffic.sync_peers.forEach(function(s) { syncMap[s.peer] = s; });
      }
      var hbMap = {};
      if (traffic && traffic.heartbeats) {
        traffic.heartbeats.forEach(function(h) { hbMap[h.peer] = h; });
      }

      var palette = M.palette ? M.palette() : {};
      var ok = palette.signalOk || '#00A651';
      var warn = palette.accent || '#FFC72C';
      var danger = palette.signalDanger || '#DC0000';
      var info = palette.info || '#4ea8de';
      var needle = danger;
      var now = (traffic && traffic.ts) || (Date.now() / 1000);

      // Build gauges for each remote peer
      var remotePeers = peers.filter(function(p) { return p.peer_name && p.peer_name !== localNode; });
      if (remotePeers.length === 0) {
        container.innerHTML = '<div style="padding:16px;color:var(--text-dim);text-align:center;">No remote peers</div>';
        return;
      }

      // Ensure layout exists
      var needsRebuild = !container.querySelector('.mesh-peer-group');
      if (needsRebuild) {
        container.innerHTML = '';
        remotePeers.forEach(function(peer) {
          var name = peer.peer_name;
          var group = document.createElement('div');
          group.className = 'mesh-peer-group';
          group.innerHTML =
            '<div class="mesh-peer-title">' + name + ' <span class="mesh-peer-os">' + (peer.os || '') + ' · ' + (peer.role || '') + '</span></div>' +
            '<div class="mesh-peer-gauges">' +
              '<canvas id="g-speed-' + name + '" width="130" height="130"></canvas>' +
              '<canvas id="g-cpu-' + name + '" width="130" height="130"></canvas>' +
              '<canvas id="g-ram-' + name + '" width="130" height="130"></canvas>' +
              '<canvas id="g-sync-' + name + '" width="130" height="130"></canvas>' +
              '<canvas id="g-bw-' + name + '" width="130" height="130"></canvas>' +
              '<canvas id="g-hb-' + name + '" width="130" height="130"></canvas>' +
            '</div>';
          container.appendChild(group);
        });
      }

      remotePeers.forEach(function(peer) {
        var name = peer.peer_name;
        var sync = syncMap[name] || {};
        var hb = hbMap[name] || {};
        var isOnline = peer.is_online;
        var lat = sync.latency_ms || 0;

        // 1. SPEED (inverse latency)
        var speed = !isOnline ? 0 : lat <= 1 ? 100 : Math.max(0, Math.round(100 - lat));
        var connType = !isOnline ? 'OFFLINE' : lat <= 3 ? 'LAN' : lat <= 30 ? 'DIRECT' : 'RELAY';
        draw('g-speed-' + name, M, {
          value: speed, max: 100, unit: connType,
          ticks: [0, 25, 50, 75, 100],
          arcColor: !isOnline ? danger : speed > 90 ? ok : speed > 50 ? warn : danger,
          bar: { value: isOnline ? Math.max(speed, 5) : 0, max: 100 },
          subLabel: lat + 'ms · Link Speed',
          needleColor: needle
        });

        // 2. CPU
        var cpu = peer.cpu || 0;
        draw('g-cpu-' + name, M, {
          value: Math.round(cpu), max: 100, unit: '%',
          ticks: [0, 25, 50, 75, 100],
          arcColor: cpu < 50 ? ok : cpu < 80 ? warn : danger,
          subLabel: 'CPU Load',
          needleColor: needle
        });

        // 3. RAM
        var memUsed = peer.mem_used_gb || 0;
        var memTotal = peer.mem_total_gb || 32;
        var memPct = Math.round((memUsed / memTotal) * 100);
        draw('g-ram-' + name, M, {
          value: memPct, max: 100, unit: '%',
          ticks: [0, 25, 50, 75, 100],
          arcColor: memPct < 60 ? ok : memPct < 85 ? warn : danger,
          bar: { value: memPct, max: 100 },
          subLabel: memUsed.toFixed(1) + '/' + memTotal.toFixed(0) + ' GB RAM',
          needleColor: needle
        });

        // 4. SYNC RATE (ops/s)
        var totalOps = (sync.total_sent || 0) + (sync.total_received || 0);
        var rate = 0;
        if (_prevSync[name]) {
          var dt = now - _prevSync[name].ts;
          if (dt > 0) rate = Math.max(0, (totalOps - _prevSync[name].ops) / dt);
        }
        _prevSync[name] = { ops: totalOps, ts: now };
        var maxRate = rate < 10 ? 20 : rate < 50 ? 100 : 500;
        draw('g-sync-' + name, M, {
          value: Math.round(rate * 10) / 10, max: maxRate, unit: 'ops/s',
          ticks: autoTicks(maxRate),
          arcColor: rate > 10 ? ok : rate > 2 ? warn : info,
          subLabel: 'Sync Rate',
          needleColor: needle
        });

        // 5. BANDWIDTH (MB/s throughput)
        var txBytes = peer.net_tx_bytes || 0;
        var rxBytes = peer.net_rx_bytes || 0;
        var totalBytes = txBytes + rxBytes;
        var bwMBs = 0;
        if (_prevNet[name]) {
          var dtN = now - _prevNet[name].ts;
          if (dtN > 0) bwMBs = Math.max(0, (totalBytes - _prevNet[name].bytes) / dtN / 1048576);
        }
        _prevNet[name] = { bytes: totalBytes, ts: now };
        var maxBw = bwMBs < 1 ? 2 : bwMBs < 10 ? 20 : 100;
        draw('g-bw-' + name, M, {
          value: Math.round(bwMBs * 100) / 100, max: maxBw, unit: 'MB/s',
          ticks: autoTicks(maxBw),
          arcColor: bwMBs > 5 ? ok : bwMBs > 0.5 ? warn : info,
          subLabel: 'TX ' + fmtBytes(txBytes) + ' · RX ' + fmtBytes(rxBytes),
          needleColor: needle
        });

        // 6. HEARTBEAT freshness
        var hbAge = hb.last_seen_ago_s != null ? hb.last_seen_ago_s : 999;
        var hbScore = !isOnline ? 0 : hbAge <= 5 ? 100 : Math.max(0, 100 - hbAge * 2);
        draw('g-hb-' + name, M, {
          value: Math.round(hbScore), max: 100, unit: isOnline ? 'ALIVE' : 'DEAD',
          ticks: [0, 25, 50, 75, 100],
          arcColor: hbScore > 80 ? ok : hbScore > 40 ? warn : danger,
          subLabel: hbAge + 's ago · Heartbeat',
          needleColor: needle
        });
      });
    }).catch(function(e) {
      if (window.DashLog) DashLog.warn('MeshMetrics', '', e.message);
    });
  }

  function draw(id, M, opts) {
    var canvas = document.getElementById(id);
    if (!canvas) return;
    canvas.getContext('2d').clearRect(0, 0, canvas.width, canvas.height);
    M.speedometer(canvas, {
      value: opts.value, max: opts.max, unit: opts.unit, size: 'fluid',
      ticks: opts.ticks, needleColor: opts.needleColor,
      arcColor: opts.arcColor, bar: opts.bar, subLabel: opts.subLabel, animate: false
    });
  }

  function autoTicks(max) {
    var step = max / 5;
    var t = [];
    for (var i = 0; i <= max; i += step) t.push(Math.round(i));
    return t;
  }

  function fmtBytes(b) {
    if (b > 1073741824) return (b / 1073741824).toFixed(1) + 'G';
    if (b > 1048576) return (b / 1048576).toFixed(0) + 'M';
    if (b > 1024) return (b / 1024).toFixed(0) + 'K';
    return b + 'B';
  }

  window.renderMeshLatency = renderMeshLatency;
})();
