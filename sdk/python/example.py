from ma_wi_api_client import Client
from ma_wi_api_client.api.chat import post_chat_completions
from ma_wi_api_client.models import ChatMessage, UnifiedChatRequest
import os

# Initialize Client
client = Client(base_url="http://127.0.0.1:8030/v1")

# Add Auth Header
# SECURITY: Never hardcode tokens in source code.
token = os.getenv("MAWI_API_TOKEN", "dev-token")
client = client.with_headers({"Authorization": f"Bearer {token}"})

# Create Request
req = UnifiedChatRequest(
    service="openai-gpt-4", messages=[ChatMessage(role="user", content="Hello via Python SDK!")], stream=False
)

print("🚀 Sending request via Python SDK...")


try:
    # Call API (sync detailed)
    from ma_wi_api_client.api.chat import post_chat_completions
    response = post_chat_completions.sync_detailed(client=client, body=req)
    
    print(f"✅ Response Status: {response.status_code}")
    if response.is_success:
        print(response.parsed.choices[0].message.content)
    else:
        print(f"❌ Failed: {response.content}")
except Exception as e:
    print(f"❌ Error: {e}")
