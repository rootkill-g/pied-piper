// Get the API base URL - same gateway that served this page
const getApiUrl = () => {
    const apiCid = document.getElementById('apiCid').value.trim();
    return `${window.location.origin}/cid/${apiCid}/api/posts`;
};

// Format date
const formatDate = (timestamp) => {
    const date = new Date(timestamp);
    return date.toLocaleString('en-US', {
        year: 'numeric',
        month: 'short',
        day: 'numeric',
        hour: '2-digit',
        minute: '2-digit'
    });
};

// Show message
const showMessage = (message, type = 'success') => {
    const msgDiv = document.createElement('div');
    msgDiv.className = type;
    msgDiv.textContent = message;
    msgDiv.style.animation = 'fadeIn 0.3s';
    
    const container = document.querySelector('.card');
    container.insertBefore(msgDiv, container.firstChild);
    
    setTimeout(() => {
        msgDiv.style.animation = 'fadeOut 0.3s';
        setTimeout(() => msgDiv.remove(), 300);
    }, 3000);
};

// Load posts
const loadPosts = async () => {
    const container = document.getElementById('postsContainer');
    container.innerHTML = '<p class="loading">Loading posts...</p>';
    
    try {
        const response = await fetch(getApiUrl());
        
        if (!response.ok) {
            throw new Error(`HTTP ${response.status}: ${response.statusText}`);
        }
        
        const posts = await response.json();
        
        if (!Array.isArray(posts)) {
            throw new Error('Invalid response format');
        }
        
        if (posts.length === 0) {
            container.innerHTML = '<p class="empty">No posts yet. Create your first post above! 📝</p>';
            return;
        }
        
        // Sort by created_at descending (newest first)
        posts.sort((a, b) => b.created_at - a.created_at);
        
        container.innerHTML = posts.map(post => `
            <div class="post" data-id="${post.id}">
                <div class="post-header">
                    <div>
                        <div class="post-title">${escapeHtml(post.title)}</div>
                        <div class="post-meta">
                            Posted: ${formatDate(post.created_at)}
                            ${post.updated_at ? ` • Updated: ${formatDate(post.updated_at)}` : ''}
                            • ID: ${post.id}
                        </div>
                    </div>
                </div>
                <div class="post-content">${escapeHtml(post.content)}</div>
                <div class="post-actions">
                    <button class="edit-btn" onclick="openEditModal('${post.id}')">✏️ Edit</button>
                    <button class="delete-btn" onclick="deletePost('${post.id}')">🗑️ Delete</button>
                </div>
            </div>
        `).join('');
        
    } catch (error) {
        console.error('Error loading posts:', error);
        container.innerHTML = `
            <div class="error">
                <strong>Error loading posts:</strong><br>
                ${error.message}<br><br>
                <small>Make sure the API CID is correct and the gateway is running.</small>
            </div>
        `;
    }
};

// Escape HTML to prevent XSS
const escapeHtml = (text) => {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
};

// Create post
document.getElementById('createForm').addEventListener('submit', async (e) => {
    e.preventDefault();
    
    const title = document.getElementById('title').value.trim();
    const content = document.getElementById('content').value.trim();
    
    if (!title || !content) {
        showMessage('Please fill in all fields', 'error');
        return;
    }
    
    const button = e.target.querySelector('button');
    const originalText = button.textContent;
    button.textContent = 'Creating...';
    button.disabled = true;
    
    try {
        const response = await fetch(getApiUrl(), {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json'
            },
            body: JSON.stringify({ title, content })
        });
        
        if (!response.ok) {
            const error = await response.json();
            throw new Error(error.error || `HTTP ${response.status}`);
        }
        
        const post = await response.json();
        
        showMessage(`Post "${post.title}" created successfully! 🎉`, 'success');
        
        // Clear form
        document.getElementById('title').value = '';
        document.getElementById('content').value = '';
        
        // Reload posts
        await loadPosts();
        
    } catch (error) {
        console.error('Error creating post:', error);
        showMessage(`Error: ${error.message}`, 'error');
    } finally {
        button.textContent = originalText;
        button.disabled = false;
    }
});

// Open edit modal
window.openEditModal = async (id) => {
    try {
        const response = await fetch(`${getApiUrl()}?id=${id}`);
        
        if (!response.ok) {
            throw new Error(`HTTP ${response.status}`);
        }
        
        const post = await response.json();
        
        document.getElementById('editId').value = post.id;
        document.getElementById('editTitle').value = post.title;
        document.getElementById('editContent').value = post.content;
        
        document.getElementById('editModal').style.display = 'block';
        
    } catch (error) {
        console.error('Error loading post:', error);
        showMessage(`Error loading post: ${error.message}`, 'error');
    }
};

// Close edit modal
window.closeEditModal = () => {
    document.getElementById('editModal').style.display = 'none';
};

// Close modal on outside click
window.onclick = (event) => {
    const modal = document.getElementById('editModal');
    if (event.target === modal) {
        closeEditModal();
    }
};

// Close modal on X click
document.querySelector('.close').onclick = closeEditModal;

// Update post
document.getElementById('editForm').addEventListener('submit', async (e) => {
    e.preventDefault();
    
    const id = document.getElementById('editId').value;
    const title = document.getElementById('editTitle').value.trim();
    const content = document.getElementById('editContent').value.trim();
    
    if (!title || !content) {
        showMessage('Please fill in all fields', 'error');
        return;
    }
    
    const button = e.target.querySelector('button[type="submit"]');
    const originalText = button.textContent;
    button.textContent = 'Updating...';
    button.disabled = true;
    
    try {
        const response = await fetch(getApiUrl(), {
            method: 'PUT',
            headers: {
                'Content-Type': 'application/json'
            },
            body: JSON.stringify({ id, title, content })
        });
        
        if (!response.ok) {
            const error = await response.json();
            throw new Error(error.error || `HTTP ${response.status}`);
        }
        
        await response.json();
        
        showMessage('Post updated successfully! ✅', 'success');
        closeEditModal();
        await loadPosts();
        
    } catch (error) {
        console.error('Error updating post:', error);
        showMessage(`Error: ${error.message}`, 'error');
    } finally {
        button.textContent = originalText;
        button.disabled = false;
    }
});

// Delete post
window.deletePost = async (id) => {
    if (!confirm('Are you sure you want to delete this post?')) {
        return;
    }
    
    try {
        const response = await fetch(`${getApiUrl()}?id=${id}`, {
            method: 'DELETE'
        });
        
        if (!response.ok) {
            const error = await response.json();
            throw new Error(error.error || `HTTP ${response.status}`);
        }
        
        showMessage('Post deleted successfully! 🗑️', 'success');
        await loadPosts();
        
    } catch (error) {
        console.error('Error deleting post:', error);
        showMessage(`Error: ${error.message}`, 'error');
    }
};

// Load posts on page load
loadPosts();

// Reload posts when API CID changes
document.getElementById('apiCid').addEventListener('change', loadPosts);
