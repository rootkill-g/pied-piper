// API Client
const api = {
    async getPosts() {
        const response = await fetch('/api/posts');
        if (!response.ok) throw new Error('Failed to fetch posts');
        return response.json();
    },

    async getPost(id) {
        const response = await fetch(`/api/posts?id=${id}`);
        if (!response.ok) throw new Error('Failed to fetch post');
        return response.json();
    },

    async createPost(title, content) {
        const response = await fetch('/api/posts', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ title, content })
        });
        if (!response.ok) throw new Error('Failed to create post');
        return response.json();
    },

    async updatePost(id, title, content) {
        const response = await fetch('/api/posts', {
            method: 'PUT',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ id, title, content })
        });
        if (!response.ok) throw new Error('Failed to update post');
        return response.json();
    },

    async deletePost(id) {
        const response = await fetch(`/api/posts?id=${id}`, {
            method: 'DELETE'
        });
        if (!response.ok) throw new Error('Failed to delete post');
        return response.json();
    }
};

// Router
class Router {
    constructor() {
        this.routes = {};
        window.addEventListener('hashchange', () => this.handleRoute());
        window.addEventListener('load', () => this.handleRoute());
    }

    on(path, handler) {
        this.routes[path] = handler;
    }

    handleRoute() {
        const hash = window.location.hash.slice(1) || '/';
        const [path, ...params] = hash.split('/').filter(Boolean);
        
        if (path === '' || path === '/') {
            this.routes['/']?.();
        } else if (path === 'post' && params[0]) {
            this.routes['/post/:id']?.(params[0]);
        } else if (path === 'new') {
            this.routes['/new']?.();
        } else if (path === 'edit' && params[0]) {
            this.routes['/edit/:id']?.(params[0]);
        } else {
            this.routes['/404']?.();
        }
    }

    navigate(path) {
        window.location.hash = path;
    }
}

// App
const app = document.getElementById('app');
const router = new Router();

// Format date
function formatDate(timestamp) {
    const date = new Date(timestamp);
    return date.toLocaleDateString('en-US', {
        year: 'numeric',
        month: 'long',
        day: 'numeric'
    });
}

// Render post list
async function renderPostList() {
    app.innerHTML = '<div class="loading">Loading posts...</div>';
    
    try {
        const posts = await api.getPosts();
        
        if (posts.length === 0) {
            app.innerHTML = `
                <div class="empty-state">
                    <h2>No posts yet</h2>
                    <p>Be the first to create a post!</p>
                    <a href="#/new" class="btn-primary">Create Post</a>
                </div>
            `;
            return;
        }

        // Sort by created date (newest first)
        posts.sort((a, b) => b.created_at - a.created_at);

        app.innerHTML = `
            <div class="post-list">
                ${posts.map(post => `
                    <article class="post-card">
                        <h2>
                            <a href="#/post/${post.id}">${escapeHtml(post.title)}</a>
                        </h2>
                        <div class="post-meta">
                            <span>📅 ${formatDate(post.created_at)}</span>
                            ${post.updated_at ? `<span>✏️ Updated ${formatDate(post.updated_at)}</span>` : ''}
                        </div>
                        <div class="post-preview">
                            ${marked.parse(post.content.substring(0, 200))}${post.content.length > 200 ? '...' : ''}
                        </div>
                        <div class="post-actions">
                            <a href="#/post/${post.id}" class="btn-secondary">Read More</a>
                            <a href="#/edit/${post.id}" class="btn-link">Edit</a>
                            <button onclick="deletePost('${post.id}')" class="btn-link delete">Delete</button>
                        </div>
                    </article>
                `).join('')}
            </div>
        `;
    } catch (error) {
        app.innerHTML = `
            <div class="error">
                <h2>Error loading posts</h2>
                <p>${error.message}</p>
                <button onclick="router.handleRoute()" class="btn-primary">Retry</button>
            </div>
        `;
    }
}

// Render single post
async function renderPost(id) {
    app.innerHTML = '<div class="loading">Loading post...</div>';
    
    try {
        const post = await api.getPost(id);
        
        app.innerHTML = `
            <article class="post-single">
                <h1>${escapeHtml(post.title)}</h1>
                <div class="post-meta">
                    <span>📅 ${formatDate(post.created_at)}</span>
                    ${post.updated_at ? `<span>✏️ Updated ${formatDate(post.updated_at)}</span>` : ''}
                </div>
                <div class="post-content">
                    ${marked.parse(post.content)}
                </div>
                <div class="post-actions">
                    <a href="#/" class="btn-secondary">← Back</a>
                    <a href="#/edit/${post.id}" class="btn-primary">Edit</a>
                    <button onclick="deletePost('${post.id}')" class="btn-link delete">Delete</button>
                </div>
            </article>
        `;
    } catch (error) {
        app.innerHTML = `
            <div class="error">
                <h2>Error loading post</h2>
                <p>${error.message}</p>
                <a href="#/" class="btn-primary">← Back to Home</a>
            </div>
        `;
    }
}

// Render new post form
function renderNewPost() {
    app.innerHTML = `
        <div class="post-form">
            <h2>Create New Post</h2>
            <form onsubmit="handleCreatePost(event)">
                <div class="form-group">
                    <label for="title">Title</label>
                    <input 
                        type="text" 
                        id="title" 
                        required 
                        maxlength="200"
                        placeholder="Enter post title..."
                    >
                </div>
                <div class="form-group">
                    <label for="content">Content (Markdown)</label>
                    <textarea 
                        id="content" 
                        required 
                        rows="15"
                        placeholder="Write your post in Markdown..."
                    ></textarea>
                </div>
                <div class="form-actions">
                    <button type="submit" class="btn-primary">Create Post</button>
                    <a href="#/" class="btn-secondary">Cancel</a>
                </div>
            </form>
            <div class="markdown-help">
                <h3>Markdown Help</h3>
                <code># Heading</code> → <strong>Heading</strong><br>
                <code>**bold**</code> → <strong>bold</strong><br>
                <code>*italic*</code> → <em>italic</em><br>
                <code>[link](url)</code> → link<br>
                <code>\`code\`</code> → <code>code</code>
            </div>
        </div>
    `;
}

// Render edit post form
async function renderEditPost(id) {
    app.innerHTML = '<div class="loading">Loading post...</div>';
    
    try {
        const post = await api.getPost(id);
        
        app.innerHTML = `
            <div class="post-form">
                <h2>Edit Post</h2>
                <form onsubmit="handleUpdatePost(event, '${id}')">
                    <div class="form-group">
                        <label for="title">Title</label>
                        <input 
                            type="text" 
                            id="title" 
                            required 
                            maxlength="200"
                            value="${escapeHtml(post.title)}"
                        >
                    </div>
                    <div class="form-group">
                        <label for="content">Content (Markdown)</label>
                        <textarea 
                            id="content" 
                            required 
                            rows="15"
                        >${escapeHtml(post.content)}</textarea>
                    </div>
                    <div class="form-actions">
                        <button type="submit" class="btn-primary">Save Changes</button>
                        <a href="#/post/${id}" class="btn-secondary">Cancel</a>
                    </div>
                </form>
            </div>
        `;
    } catch (error) {
        app.innerHTML = `
            <div class="error">
                <h2>Error loading post</h2>
                <p>${error.message}</p>
                <a href="#/" class="btn-primary">← Back to Home</a>
            </div>
        `;
    }
}

// Handle create post
async function handleCreatePost(event) {
    event.preventDefault();
    
    const title = document.getElementById('title').value;
    const content = document.getElementById('content').value;
    
    try {
        const post = await api.createPost(title, content);
        router.navigate(`/post/${post.id}`);
    } catch (error) {
        alert(`Error creating post: ${error.message}`);
    }
}

// Handle update post
async function handleUpdatePost(event, id) {
    event.preventDefault();
    
    const title = document.getElementById('title').value;
    const content = document.getElementById('content').value;
    
    try {
        await api.updatePost(id, title, content);
        router.navigate(`/post/${id}`);
    } catch (error) {
        alert(`Error updating post: ${error.message}`);
    }
}

// Handle delete post
async function deletePost(id) {
    if (!confirm('Are you sure you want to delete this post?')) {
        return;
    }
    
    try {
        await api.deletePost(id);
        router.navigate('/');
    } catch (error) {
        alert(`Error deleting post: ${error.message}`);
    }
}

// Utility function
function escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
}

// Routes
router.on('/', renderPostList);
router.on('/post/:id', renderPost);
router.on('/new', renderNewPost);
router.on('/edit/:id', renderEditPost);
router.on('/404', () => {
    app.innerHTML = `
        <div class="error">
            <h2>404 - Page Not Found</h2>
            <a href="#/" class="btn-primary">← Back to Home</a>
        </div>
    `;
});

// Configure marked
if (typeof marked !== 'undefined') {
    marked.setOptions({
        breaks: true,
        gfm: true,
        headerIds: true,
        mangle: false
    });
}
