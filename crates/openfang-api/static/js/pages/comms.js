// OpenFang Comms Page — Agent topology & inter-agent communication feed
'use strict';

function commsPage() {
  return {
    topology: { nodes: [], edges: [] },
    events: [],
    loading: true,
    loadError: '',
    sseSource: null,
    streamConnected: false,
    showSendModal: false,
    showTaskModal: false,
    sendMode: 'agent',
    sendFrom: '',
    sendTo: '',
    sendChannel: '',
    sendRecipient: '',
    sendThreadId: '',
    sendMsg: '',
    sendAttachments: [],
    sendLoading: false,
    taskTitle: '',
    taskDesc: '',
    taskAssign: '',
    taskLoading: false,

    async loadData() {
      this.loading = true;
      this.loadError = '';
      try {
        var results = await Promise.all([
          OpenFangAPI.get('/api/comms/topology'),
          OpenFangAPI.get('/api/comms/events?limit=200')
        ]);
        this.topology = results[0] || { nodes: [], edges: [] };
        this.events = results[1] || [];
        this.startSSE();
      } catch(e) {
        this.stopSSE();
        this.loadError = e.message || 'Could not load comms data.';
      }
      this.loading = false;
    },

    startSSE() {
      if (this.sseSource) this.sseSource.close();
      var self = this;
      var url = window.location.origin + '/api/comms/events/stream';
      var token = OpenFangAPI.getToken();
      if (token) url += '?token=' + encodeURIComponent(token);
      this.sseSource = new EventSource(url);
      this.streamConnected = false;
      this.sseSource.onopen = function() {
        self.streamConnected = true;
      };
      this.sseSource.onmessage = function(ev) {
        if (ev.data === 'ping') return;
        try {
          var event = JSON.parse(ev.data);
          self.events.unshift(event);
          if (self.events.length > 200) self.events.length = 200;
          // Refresh topology on spawn/terminate events
          if (event.kind === 'agent_spawned' || event.kind === 'agent_terminated') {
            self.refreshTopology();
          }
        } catch(e) { /* ignore parse errors */ }
      };
      this.sseSource.onerror = function() {
        self.streamConnected = false;
        self.stopSSE();
      };
    },

    stopSSE() {
      if (this.sseSource) {
        this.sseSource.close();
        this.sseSource = null;
      }
      this.streamConnected = false;
    },

    async refreshTopology() {
      try {
        this.topology = await OpenFangAPI.get('/api/comms/topology');
      } catch(e) { /* silent */ }
    },

    rootNodes() {
      var childIds = {};
      var self = this;
      this.topology.edges.forEach(function(e) {
        if (e.kind === 'parent_child') childIds[e.to] = true;
      });
      return this.topology.nodes.filter(function(n) { return !childIds[n.id]; });
    },

    childrenOf(id) {
      var childIds = {};
      this.topology.edges.forEach(function(e) {
        if (e.kind === 'parent_child' && e.from === id) childIds[e.to] = true;
      });
      return this.topology.nodes.filter(function(n) { return childIds[n.id]; });
    },

    peersOf(id) {
      var peerIds = {};
      this.topology.edges.forEach(function(e) {
        if (e.kind === 'peer') {
          if (e.from === id) peerIds[e.to] = true;
          if (e.to === id) peerIds[e.from] = true;
        }
      });
      return this.topology.nodes.filter(function(n) { return peerIds[n.id]; });
    },

    stateBadgeClass(state) {
      switch(state) {
        case 'Running': return 'badge badge-success';
        case 'Suspended': return 'badge badge-warning';
        case 'Terminated': case 'Crashed': return 'badge badge-danger';
        default: return 'badge badge-dim';
      }
    },

    eventBadgeClass(kind) {
      switch(kind) {
        case 'agent_message': return 'badge badge-info';
        case 'agent_spawned': return 'badge badge-success';
        case 'agent_terminated': return 'badge badge-danger';
        case 'task_posted': return 'badge badge-warning';
        case 'task_claimed': return 'badge badge-info';
        case 'task_completed': return 'badge badge-success';
        default: return 'badge badge-dim';
      }
    },

    eventIcon(kind) {
      switch(kind) {
        case 'agent_message': return '\u2709';
        case 'agent_spawned': return '+';
        case 'agent_terminated': return '\u2715';
        case 'task_posted': return '\u2691';
        case 'task_claimed': return '\u2690';
        case 'task_completed': return '\u2713';
        default: return '\u2022';
      }
    },

    eventLabel(kind) {
      switch(kind) {
        case 'agent_message': return 'Message';
        case 'agent_spawned': return 'Spawned';
        case 'agent_terminated': return 'Terminated';
        case 'task_posted': return 'Task Posted';
        case 'task_claimed': return 'Task Claimed';
        case 'task_completed': return 'Task Done';
        default: return kind;
      }
    },

    timeAgo(dateStr) {
      if (!dateStr) return '';
      var d = new Date(dateStr);
      var secs = Math.floor((Date.now() - d.getTime()) / 1000);
      if (secs < 60) return secs + 's ago';
      if (secs < 3600) return Math.floor(secs / 60) + 'm ago';
      if (secs < 86400) return Math.floor(secs / 3600) + 'h ago';
      return Math.floor(secs / 86400) + 'd ago';
    },

    openSendModal() {
      this.sendMode = 'agent';
      this.sendFrom = '';
      this.sendTo = '';
      this.sendChannel = '';
      this.sendRecipient = '';
      this.sendThreadId = '';
      this.sendMsg = '';
      this.sendAttachments = [];
      this.showSendModal = true;
    },

    canSubmitSend() {
      if (!this.sendFrom) return false;
      if (!this.sendMsg.trim() && !this.sendAttachments.length) return false;
      if (this.sendMode === 'agent') return !!this.sendTo;
      return !!this.sendChannel;
    },

    handleSendFiles(event) {
      var files = Array.from((event && event.target && event.target.files) || []);
      for (var i = 0; i < files.length; i++) {
        this.sendAttachments.push({ file: files[i], uploading: false });
      }
      if (event && event.target) event.target.value = '';
    },

    removeSendAttachment(idx) {
      this.sendAttachments.splice(idx, 1);
    },

    async submitSend() {
      if (!this.canSubmitSend()) return;
      this.sendLoading = true;
      try {
        var uploadedFiles = [];
        for (var i = 0; i < this.sendAttachments.length; i++) {
          var att = this.sendAttachments[i];
          att.uploading = true;
          try {
            var uploadRes = await OpenFangAPI.upload(this.sendFrom, att.file);
            uploadedFiles.push({
              file_id: uploadRes.file_id,
              filename: uploadRes.filename,
              content_type: uploadRes.content_type
            });
          } catch(e) {
            OpenFangToast.error('Failed to upload ' + att.file.name);
            this.sendLoading = false;
            att.uploading = false;
            return;
          }
          att.uploading = false;
        }

        var body = {
          from_agent_id: this.sendFrom,
          message: this.sendMsg,
          attachments: uploadedFiles
        };
        if (this.sendThreadId) body.thread_id = this.sendThreadId;
        if (this.sendMode === 'agent') {
          body.to_agent_id = this.sendTo;
        } else {
          body.channel = this.sendChannel;
          if (this.sendRecipient) body.recipient = this.sendRecipient;
        }

        await OpenFangAPI.post('/api/comms/send', body);
        OpenFangToast.success('Message sent');
        this.showSendModal = false;
      } catch(e) {
        OpenFangToast.error(e.message || 'Send failed');
      }
      this.sendLoading = false;
    },

    openTaskModal() {
      this.taskTitle = '';
      this.taskDesc = '';
      this.taskAssign = '';
      this.showTaskModal = true;
    },

    async submitTask() {
      if (!this.taskTitle.trim()) return;
      this.taskLoading = true;
      try {
        var body = { title: this.taskTitle, description: this.taskDesc };
        if (this.taskAssign) body.assigned_to = this.taskAssign;
        await OpenFangAPI.post('/api/comms/task', body);
        OpenFangToast.success('Task posted');
        this.showTaskModal = false;
      } catch(e) {
        OpenFangToast.error(e.message || 'Task failed');
      }
      this.taskLoading = false;
    }
  };
}
