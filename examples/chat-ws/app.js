let ws = null;
let username = null;
let users = new Set();

const messagesDiv = document.getElementById('messages');
const messageInput = document.getElementById('messageInput');
const sendButton = document.getElementById('sendButton');
const statusDiv = document.getElementById('status');
const usernameInput = document.getElementById('usernameInput');
const joinButton = document.getElementById('joinButton');
const joinOverlay = document.getElementById('joinOverlay');
const userList = document.getElementById('userList');

function connect() {
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const wsUrl = `${protocol}//${window.location.host}/ws/app/chat`;
    
    ws = new WebSocket(wsUrl);
    
    ws.onopen = () => {
        updateStatus('Connected', 'connected');
        enableChat();
    };
    
    ws.onclose = () => {
        updateStatus('Disconnected', 'disconnected');
        disableChat();
        
        // Attempt reconnect after 3 seconds
        setTimeout(connect, 3000);
    };
    
    ws.onerror = (error) => {
        console.error('WebSocket error:', error);
        updateStatus('Error', 'error');
    };
    
    ws.onmessage = (event) => {
        try {
            const msg = JSON.parse(event.data);
            handleServerMessage(msg);
        } catch (e) {
            console.error('Failed to parse message:', e);
        }
    };
}

function handleServerMessage(msg) {
    switch (msg.type) {
        case 'history':
            displayHistory(msg.messages);
            break;
        case 'join':
            addSystemMessage(`${msg.username} joined the chat`);
            users.add(msg.username);
            updateUserList();
            break;
        case 'leave':
            addSystemMessage(`${msg.username} left the chat`);
            users.delete(msg.username);
            updateUserList();
            break;
        case 'message':
            addChatMessage(msg.username, msg.text, msg.timestamp);
            break;
        case 'error':
            addSystemMessage(`Error: ${msg.message}`, true);
            break;
    }
}

function displayHistory(messages) {
    messagesDiv.innerHTML = '';
    messages.forEach(msg => {
        addChatMessage(msg.username, msg.text, msg.timestamp, false);
    });
    scrollToBottom();
}

function addChatMessage(user, text, timestamp, scroll = true) {
    const msgDiv = document.createElement('div');
    msgDiv.className = 'message';
    
    if (user === username) {
        msgDiv.classList.add('own-message');
    }
    
    const time = new Date(timestamp).toLocaleTimeString();
    
    msgDiv.innerHTML = `
        <div class="message-header">
            <strong>${escapeHtml(user)}</strong>
            <span class="timestamp">${time}</span>
        </div>
        <div class="message-text">${escapeHtml(text)}</div>
    `;
    
    messagesDiv.appendChild(msgDiv);
    
    if (scroll) {
        scrollToBottom();
    }
}

function addSystemMessage(text, isError = false) {
    const msgDiv = document.createElement('div');
    msgDiv.className = 'message system-message';
    if (isError) {
        msgDiv.classList.add('error-message');
    }
    msgDiv.textContent = text;
    messagesDiv.appendChild(msgDiv);
    scrollToBottom();
}

function sendMessage() {
    const text = messageInput.value.trim();
    if (!text || !ws || ws.readyState !== WebSocket.OPEN) return;
    
    ws.send(JSON.stringify({
        type: 'message',
        text: text
    }));
    
    messageInput.value = '';
    messageInput.focus();
}

function joinChat() {
    username = usernameInput.value.trim();
    if (!username) {
        alert('Please enter a username');
        return;
    }
    
    if (ws && ws.readyState === WebSocket.OPEN) {
        ws.send(JSON.stringify({
            type: 'join',
            username: username
        }));
        
        joinOverlay.style.display = 'none';
        users.add(username);
        updateUserList();
    } else {
        alert('Not connected to server. Please wait...');
    }
}

function leaveChat() {
    if (ws && ws.readyState === WebSocket.OPEN) {
        ws.send(JSON.stringify({
            type: 'leave'
        }));
    }
}

function updateStatus(text, className) {
    statusDiv.textContent = text;
    statusDiv.className = 'status ' + className;
}

function enableChat() {
    messageInput.disabled = false;
    sendButton.disabled = false;
}

function disableChat() {
    messageInput.disabled = true;
    sendButton.disabled = true;
}

function scrollToBottom() {
    messagesDiv.scrollTop = messagesDiv.scrollHeight;
}

function updateUserList() {
    userList.innerHTML = '';
    Array.from(users).sort().forEach(user => {
        const li = document.createElement('li');
        li.textContent = user;
        if (user === username) {
            li.classList.add('current-user');
        }
        userList.appendChild(li);
    });
}

function escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
}

// Event listeners
sendButton.addEventListener('click', sendMessage);

messageInput.addEventListener('keypress', (e) => {
    if (e.key === 'Enter') {
        sendMessage();
    }
});

joinButton.addEventListener('click', joinChat);

usernameInput.addEventListener('keypress', (e) => {
    if (e.key === 'Enter') {
        joinChat();
    }
});

window.addEventListener('beforeunload', () => {
    leaveChat();
});

// Initialize
connect();
usernameInput.focus();
