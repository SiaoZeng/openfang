// OpenFang Builder Page — capability-gap analysis, approval, and apply job tracking
'use strict';

function builderPage() {
  return {
    goalInput: '',
    analysis: null,
    analyzing: false,
    applying: false,
    loading: true,
    loadError: '',
    jobs: [],
    pendingApprovals: [],
    currentJob: null,
    currentApprovalId: '',
    capabilitiesTotal: 0,

    async loadData() {
      this.loading = true;
      this.loadError = '';
      try {
        await Promise.all([
          this.loadJobs(),
          this.loadPendingApprovals(),
          this.loadCapabilitiesCount()
        ]);
        await this.restoreLastJob();
      } catch (e) {
        this.capabilitiesTotal = 0;
        this.pendingApprovals = [];
        this.jobs = [];
        this.loadError = e.message || 'Could not load builder state.';
      }
      this.loading = false;
    },

    async loadCapabilitiesCount() {
      var data = await OpenFangAPI.get('/api/routing/capabilities');
      this.capabilitiesTotal = data.total || 0;
    },

    async loadJobs() {
      var data = await OpenFangAPI.get('/api/routing/proposals/jobs');
      this.jobs = data.jobs || [];
    },

    async loadPendingApprovals() {
      var data = await OpenFangAPI.get('/api/approvals');
      this.pendingApprovals = (data.approvals || []).filter(function(a) {
        return a.tool_name === 'capability_apply';
      });
    },

    async restoreLastJob() {
      var lastJobId = localStorage.getItem('openfang-builder-last-job');
      if (!lastJobId) return;
      try {
        await this.fetchJob(lastJobId, true);
      } catch (e) {
        localStorage.removeItem('openfang-builder-last-job');
        this.currentJob = null;
      }
    },

    async analyzeGoal() {
      if (!this.goalInput || !this.goalInput.trim()) {
        OpenFangToast.warn('Enter a goal to analyze first.');
        return;
      }
      this.analyzing = true;
      this.analysis = null;
      try {
        this.analysis = await OpenFangAPI.post('/api/routing/proposals', {
          message: this.goalInput.trim()
        });
        if (this.analysis.gap_detected) {
          OpenFangToast.success('Draft proposal generated');
        } else {
          OpenFangToast.info('Existing capabilities look sufficient for this goal');
        }
      } catch (e) {
        OpenFangToast.error(e.message || 'Analysis failed');
      }
      this.analyzing = false;
    },

    async applyProposal(activateAfterCreate) {
      if (!this.analysis || !this.analysis.proposal) {
        OpenFangToast.warn('Generate a proposal first.');
        return;
      }
      this.applying = true;
      try {
        var result = await OpenFangAPI.post('/api/routing/proposals/apply', {
          proposal: this.analysis.proposal,
          activate_after_create: !!activateAfterCreate
        });
        this.currentApprovalId = result.approval_id || '';
        if (!result.job_id) throw new Error(result.error || 'Apply failed: no job_id returned');
        localStorage.setItem('openfang-builder-last-job', result.job_id);
        await this.fetchJob(result.job_id, true);
        await this.loadPendingApprovals();
        await this.loadJobs();
        OpenFangToast.success('Proposal submitted for approval');
      } catch (e) {
        OpenFangToast.error(e.message || 'Apply failed');
      }
      this.applying = false;
    },

    async fetchJob(jobId, makeCurrent) {
      var job = await OpenFangAPI.get('/api/routing/proposals/jobs/' + jobId);
      if (makeCurrent !== false) this.currentJob = job;
      if (job && job.approval_id) this.currentApprovalId = job.approval_id;
      return job;
    },

    async refreshCurrentJob() {
      if (!this.currentJob || !this.currentJob.job_id) return;
      try {
        await this.fetchJob(this.currentJob.job_id, true);
        await this.loadJobs();
        await this.loadPendingApprovals();
      } catch (e) {
        OpenFangToast.error(e.message || 'Could not refresh job');
      }
    },

    async approveCurrent() {
      if (!this.currentApprovalId) return;
      try {
        await OpenFangAPI.post('/api/approvals/' + this.currentApprovalId + '/approve', {});
        OpenFangToast.success('Proposal approved');
        await this.pollCurrentJob();
      } catch (e) {
        OpenFangToast.error(e.message || 'Approval failed');
      }
    },

    async rejectCurrent() {
      var self = this;
      if (!this.currentApprovalId) return;
      OpenFangToast.confirm(
        'Reject Proposal',
        'Reject this capability proposal?',
        async function() {
          try {
            await OpenFangAPI.post('/api/approvals/' + self.currentApprovalId + '/reject', {});
            OpenFangToast.success('Proposal rejected');
            await self.pollCurrentJob();
          } catch (e) {
            OpenFangToast.error(e.message || 'Reject failed');
          }
        }
      );
    },

    async pollCurrentJob() {
      if (!this.currentJob || !this.currentJob.job_id) return;
      for (var i = 0; i < 60; i++) {
        await this.refreshCurrentJob();
        if (this.isTerminalStatus(this.currentJob && this.currentJob.status)) return;
        await new Promise(function(resolve) { setTimeout(resolve, 500); });
      }
      if (this.currentJob && !this.isTerminalStatus(this.currentJob.status)) {
        OpenFangToast.warn('Job still in progress — use refresh to check status.');
      }
    },

    isTerminalStatus(status) {
      return ['applied', 'rejected', 'timed_out', 'failed'].indexOf(status) >= 0;
    },

    statusBadgeClass(status) {
      if (status === 'applied') return 'badge-success';
      if (status === 'pending_approval' || status === 'applying') return 'badge-warn';
      if (status === 'rejected' || status === 'failed' || status === 'timed_out') return 'badge-error';
      return 'badge-muted';
    },

    formatLabel(value) {
      return (value || '')
        .replace(/_/g, ' ')
        .replace(/\b\w/g, function(ch) { return ch.toUpperCase(); });
    },

    topMatchSummary(match) {
      if (!match) return '';
      var keywords = (match.matched_keywords || []).join(', ');
      return match.name + ' (' + match.score.toFixed(2) + (keywords ? '; ' + keywords : '') + ')';
    },

    proposalArtifact(proposal) {
      if (!proposal) return '';
      if (proposal.kind === 'agent') return proposal.agent_manifest_toml || '';
      if (proposal.kind === 'hand') return proposal.hand_toml || '';
      if (proposal.kind === 'workflow' && proposal.workflow) {
        return JSON.stringify(proposal.workflow, null, 2);
      }
      return '';
    },

    outcomeSummary(job) {
      if (!job || !job.outcome) return '';
      if (job.outcome.kind === 'workflow') {
        return 'Workflow "' + job.outcome.name + '" created';
      }
      if (job.outcome.kind === 'agent') {
        return 'Agent "' + job.outcome.name + '" created';
      }
      if (job.outcome.kind === 'hand') {
        return 'Hand "' + job.outcome.hand_id + '"' + (job.outcome.activated ? ' created and activated' : ' created');
      }
      return '';
    },

    useJob(job) {
      this.currentJob = job;
      this.currentApprovalId = job.approval_id || '';
      localStorage.setItem('openfang-builder-last-job', job.job_id);
    }
  };
}
