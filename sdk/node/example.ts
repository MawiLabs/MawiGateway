import { MaWiClient } from './MaWiClient';

// Initialize Client
const token = process.env.MAWI_API_TOKEN || 'dev-token';

const client = new MaWiClient({
    BASE: 'http://127.0.0.1:8030/v1',
    HEADERS: {
        'Authorization': `Bearer ${token}`
    }
});

async function main() {
    console.log("🚀 Sending request via Node SDK...");
    try {
        const response = await client.chat.postChatCompletions({
            service: 'openai-gpt-4',
            messages: [{ role: 'user', content: 'Hello via Node SDK!' }],
            stream: false
        });

        const data = response as any;
        console.log("✅ Response Received:");
        if (data.choices && data.choices.length > 0) {
            console.log(data.choices[0].message.content);
        } else {
            console.log(data);
        }
    } catch (error) {
        console.error('❌ Error:', error);
    }
}

main();
