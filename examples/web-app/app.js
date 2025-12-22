// Display CID from URL
window.addEventListener('DOMContentLoaded', () => {
    const cidElement = document.getElementById('cid');
    const path = window.location.pathname;
    
    // Extract CID from URL path (/cid/<cid>/...)
    const cidMatch = path.match(/\/cid\/([^\/]+)/);
    if (cidMatch && cidMatch[1]) {
        cidElement.textContent = cidMatch[1];
    } else {
        cidElement.textContent = 'Unknown (not accessed via CID)';
    }
    
    console.log('Pied Piper Web App loaded!');
    console.log('Path:', path);
});

// Test API call function
async function fetchData() {
    const resultDiv = document.getElementById('result');
    resultDiv.textContent = 'Loading...';
    
    try {
        // This would call a WASM backend API
        // For now, just demonstrate the concept
        resultDiv.innerHTML = `
            <strong>API Call Example:</strong><br>
            <span style="color: #10b981;">✓ Connection successful</span><br>
            <br>
            <em>Note: Full WASM backend API integration coming soon!</em><br>
            <br>
            In a complete setup, this button would trigger a POST request to:<br>
            <code>/cid/&lt;cid&gt;/api/endpoint</code><br>
            <br>
            The request would be routed to a WASM handler that processes the data<br>
            and returns a JSON response.
        `;
        
    } catch (error) {
        resultDiv.innerHTML = `
            <strong style="color: #ef4444;">Error:</strong><br>
            ${error.message}
        `;
    }
}

// Add some interactivity
document.querySelectorAll('.card').forEach(card => {
    card.addEventListener('click', (e) => {
        if (e.target.tagName !== 'BUTTON') {
            card.style.transform = 'scale(1.02)';
            setTimeout(() => {
                card.style.transform = '';
            }, 200);
        }
    });
});

console.log('🚀 Pied Piper Demo App initialized');
console.log('Version: 1.0.0');
console.log('Network: Decentralized');
