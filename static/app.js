class GitCloner {
    constructor() {
        this.token = localStorage.getItem('token');
        this.username = localStorage.getItem('username');
        this.currentPage = 1;
        this.itemsPerPage = 10;
        this.init();
    }

    init() {
        this.bindEvents();
        if (this.token) {
            this.showApp();
            this.loadRepositories();
        } else {
            this.showLogin();
        }
    }

    bindEvents() {
        // Login form
        document.getElementById('loginForm').addEventListener('submit', (e) => {
            e.preventDefault();
            this.login();
        });

        // Add repository form
        document.getElementById('addRepoForm').addEventListener('submit', (e) => {
            e.preventDefault();
            this.addRepository();
        });

        // Logout button
        document.getElementById('logoutBtn').addEventListener('click', () => {
            this.logout();
        });
    }

    async login() {
        const username = document.getElementById('username').value;
        const password = document.getElementById('password').value;

        try {
            const response = await fetch('/api/auth/login', {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify({ username, password }),
            });

            const data = await response.json();

            if (data.success) {
                this.token = data.data.token;
                this.username = data.data.username;
                localStorage.setItem('token', this.token);
                localStorage.setItem('username', this.username);
                this.showApp();
                this.loadRepositories();
            } else {
                this.showAlert('loginAlert', data.message || 'Login failed', 'error');
            }
        } catch (error) {
            this.showAlert('loginAlert', 'Network error: ' + error.message, 'error');
        }
    }

    logout() {
        this.token = null;
        this.username = null;
        localStorage.removeItem('token');
        localStorage.removeItem('username');
        this.showLogin();
    }

    async addRepository() {
        const repoUrl = document.getElementById('repoUrl').value;
        const form = document.getElementById('addRepoForm');
        const submitBtn = form.querySelector('button[type="submit"]');
        
        // Show loading state
        submitBtn.innerHTML = '<span class="spinner"></span>Cloning...';
        submitBtn.disabled = true;

        try {
            const response = await fetch('/api/repositories', {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                    'Authorization': `Bearer ${this.token}`,
                },
                body: JSON.stringify({ url: repoUrl }),
            });

            const data = await response.json();

            if (data.success) {
                this.showAlert('appAlert', data.message || 'Repository cloned successfully', 'success');
                document.getElementById('repoUrl').value = '';
                this.loadRepositories();
            } else {
                this.showAlert('appAlert', data.message || 'Failed to clone repository', 'error');
            }
        } catch (error) {
            this.showAlert('appAlert', 'Network error: ' + error.message, 'error');
        } finally {
            // Reset button state
            submitBtn.innerHTML = 'Clone Repository';
            submitBtn.disabled = false;
        }
    }

    async loadRepositories(page = 1) {
        this.currentPage = page;
        const listContainer = document.getElementById('repositoriesList');
        listContainer.innerHTML = '<div style="padding: 20px; text-align: center;">Loading repositories...</div>';

        try {
            const response = await fetch(`/api/repositories?page=${page}&limit=${this.itemsPerPage}`, {
                headers: {
                    'Authorization': `Bearer ${this.token}`,
                },
            });

            const data = await response.json();

            if (data.success) {
                this.renderRepositories(data.data);
            } else {
                listContainer.innerHTML = '<div style="padding: 20px; text-align: center; color: red;">Failed to load repositories</div>';
            }
        } catch (error) {
            listContainer.innerHTML = '<div style="padding: 20px; text-align: center; color: red;">Network error: ' + error.message + '</div>';
        }
    }

    renderRepositories(paginatedData) {
        const listContainer = document.getElementById('repositoriesList');
        const repositories = paginatedData.items;

        if (repositories.length === 0) {
            listContainer.innerHTML = `
                <div class="empty-state">
                    <h4>No repositories cloned yet</h4>
                    <p>Add your first repository using the form above</p>
                </div>
            `;
            return;
        }

        const repositoriesHtml = repositories.map(repo => `
            <div class="repo-item" data-url="${encodeURIComponent(repo.url)}">
                <div class="repo-info">
                    <div class="repo-name">${this.escapeHtml(repo.name)}</div>
                    <div class="repo-url">${this.escapeHtml(repo.url)}</div>
                    <div class="repo-meta">
                        <span class="repo-status status-${repo.status}">${repo.status}</span>
                        ${repo.last_synced ? `• Last synced: ${new Date(repo.last_synced).toLocaleString()}` : '• Never synced'}
                    </div>
                </div>
                <div class="repo-actions">
                    <button class="btn btn-success btn-small sync-btn" data-url="${encodeURIComponent(repo.url)}">
                        Sync
                    </button>
                    <button class="btn btn-danger btn-small remove-btn" data-url="${encodeURIComponent(repo.url)}">
                        Remove
                    </button>
                </div>
            </div>
        `).join('');

        // Create pagination controls
        const paginationHtml = this.createPaginationControls(paginatedData);
        
        listContainer.innerHTML = repositoriesHtml + paginationHtml;

        // Bind action buttons
        this.bindRepositoryActions();
    }

    createPaginationControls(paginatedData) {
        const { page, total_pages, total } = paginatedData;
        
        if (total_pages <= 1) {
            return '<div class="pagination-info">Showing all ' + total + ' repositories</div>';
        }

        let paginationHtml = '<div class="pagination-container">';
        paginationHtml += '<div class="pagination-info">Page ' + page + ' of ' + total_pages + ' (' + total + ' total)</div>';
        paginationHtml += '<div class="pagination-controls">';
        
        // Previous button
        if (page > 1) {
            paginationHtml += '<button class="btn btn-small pagination-btn" data-page="' + (page - 1) + '">Previous</button>';
        }
        
        // Page numbers (show max 5 pages around current)
        const startPage = Math.max(1, page - 2);
        const endPage = Math.min(total_pages, page + 2);
        
        for (let p = startPage; p <= endPage; p++) {
            const activeClass = p === page ? ' active' : '';
            paginationHtml += '<button class="btn btn-small pagination-btn' + activeClass + '" data-page="' + p + '">' + p + '</button>';
        }
        
        // Next button
        if (page < total_pages) {
            paginationHtml += '<button class="btn btn-small pagination-btn" data-page="' + (page + 1) + '">Next</button>';
        }
        
        paginationHtml += '</div></div>';
        return paginationHtml;
    }

    bindRepositoryActions() {
        // Sync buttons
        document.querySelectorAll('.sync-btn').forEach(btn => {
            btn.addEventListener('click', (e) => {
                const url = e.target.dataset.url;
                this.syncRepository(url, e.target);
            });
        });

        // Remove buttons
        document.querySelectorAll('.remove-btn').forEach(btn => {
            btn.addEventListener('click', (e) => {
                const url = e.target.dataset.url;
                if (confirm('Are you sure you want to remove this repository?')) {
                    this.removeRepository(url);
                }
            });
        });

        // Pagination buttons
        document.querySelectorAll('.pagination-btn').forEach(btn => {
            btn.addEventListener('click', (e) => {
                const page = parseInt(e.target.dataset.page);
                if (page && page !== this.currentPage) {
                    this.loadRepositories(page);
                }
            });
        });
    }

    async syncRepository(encodedUrl, button) {
        const originalText = button.innerHTML;
        button.innerHTML = '<span class="spinner"></span>Syncing...';
        button.disabled = true;

        try {
            const response = await fetch(`/api/repositories/${encodedUrl}/sync`, {
                method: 'POST',
                headers: {
                    'Authorization': `Bearer ${this.token}`,
                },
            });

            const data = await response.json();

            if (data.success) {
                this.showAlert('appAlert', 'Repository synced successfully', 'success');
                this.loadRepositories();
            } else {
                this.showAlert('appAlert', data.message || 'Failed to sync repository', 'error');
            }
        } catch (error) {
            this.showAlert('appAlert', 'Network error: ' + error.message, 'error');
        } finally {
            button.innerHTML = originalText;
            button.disabled = false;
        }
    }

    async removeRepository(encodedUrl) {
        try {
            const response = await fetch(`/api/repositories/${encodedUrl}`, {
                method: 'DELETE',
                headers: {
                    'Authorization': `Bearer ${this.token}`,
                },
            });

            const data = await response.json();

            if (data.success) {
                this.showAlert('appAlert', 'Repository removed successfully', 'success');
                this.loadRepositories();
            } else {
                this.showAlert('appAlert', data.message || 'Failed to remove repository', 'error');
            }
        } catch (error) {
            this.showAlert('appAlert', 'Network error: ' + error.message, 'error');
        }
    }

    showLogin() {
        document.getElementById('loginContainer').classList.remove('hidden');
        document.getElementById('appContainer').classList.add('hidden');
        this.hideAlert('loginAlert');
    }

    showApp() {
        document.getElementById('loginContainer').classList.add('hidden');
        document.getElementById('appContainer').classList.remove('hidden');
        document.getElementById('userInfo').textContent = `Welcome, ${this.username}`;
        this.hideAlert('appAlert');
    }

    showAlert(alertId, message, type) {
        const alert = document.getElementById(alertId);
        alert.className = `alert alert-${type}`;
        alert.textContent = message;
        alert.classList.remove('hidden');

        // Auto-hide success alerts after 5 seconds
        if (type === 'success') {
            setTimeout(() => {
                this.hideAlert(alertId);
            }, 5000);
        }
    }

    hideAlert(alertId) {
        document.getElementById(alertId).classList.add('hidden');
    }

    escapeHtml(text) {
        const div = document.createElement('div');
        div.textContent = text;
        return div.innerHTML;
    }
}

// Initialize the app when the page loads
document.addEventListener('DOMContentLoaded', () => {
    new GitCloner();
});
