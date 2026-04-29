-- Migration 036: Add video + image + audio generation providers
-- Seeds providers and flagship models for the multimodal roadmap:
--   - xAI Grok Imagine (image + video, extends existing xai)
--   - Runway Gen-4 / Gen-4.5 (video)
--   - Kuaishou Kling (video)
--   - Luma AI Dream Machine (video)
--   - Pika Labs (video)
--   - MiniMax Hailuo (video + chat)
--   - ByteDance Seedance (video)
--   - Hume AI Octave + EVI (audio: TTS + speech-to-speech)
--
-- Pricing fields use cost_per_1k_input_tokens for an approximate per-second
-- USD cost; downstream cost-tracking will need a per-second column eventually.
-- For now we encode published "per-second" pricing in cost_per_1k_input_tokens
-- (treated as a flat per-call multiplier by the existing cost tracker).

INSERT INTO providers (id, name, provider_type, api_endpoint, description) VALUES
    ('runway',    'Runway',           'runway',    'https://api.dev.runwayml.com/v1',                'Runway Gen-4 / Gen-4.5 video generation'),
    ('kling',     'Kuaishou Kling',   'kling',     'https://api.klingai.com/v1',                     'Kuaishou Kling high-fidelity video'),
    ('lumaai',    'Luma AI',          'lumaai',    'https://api.lumalabs.ai/dream-machine/v1',       'Luma AI Dream Machine video'),
    ('pika',      'Pika Labs',        'pika',      'https://api.pika.art/v1',                        'Pika Labs stylized video'),
    ('minimax',   'MiniMax',          'minimax',   'https://api.minimax.chat/v1',                    'MiniMax Hailuo video + Abab chat'),
    ('bytedance', 'ByteDance',        'bytedance', 'https://ark.cn-beijing.volces.com/api/v3',       'ByteDance Seedance video (via Ark)'),
    ('hume',      'Hume AI',          'hume',      'https://api.hume.ai/v0',                         'Hume AI Octave TTS + EVI empathic voice')
ON CONFLICT (id) DO NOTHING;

-- xAI provider already exists (provider_type='xai'); no row needed.

-- Flagship models. worker_type='video' / 'image' lets the routing layer
-- pre-filter pools by capability without parsing model names.
INSERT INTO models (id, name, provider_id, modality, worker_type, tier_required, cost_per_1k_input_tokens, cost_per_1k_output_tokens, tier) VALUES
    -- xAI Grok Imagine
    ('xai-grok-imagine-v1',     'grok-imagine',          'xai',       'image+video', 'video', 'A', 0.080, 0.0, 'premium'),

    -- Runway Gen-4 / 4.5
    ('runway-gen-4',            'gen-4',                 'runway',    'video',       'video', 'A', 0.250, 0.0, 'premium'),
    ('runway-gen-4-5',          'gen-4.5',               'runway',    'video',       'video', 'A', 0.300, 0.0, 'premium'),

    -- Kuaishou Kling
    ('kling-v1-5',              'kling-v1-5',            'kling',     'video',       'video', 'A', 0.140, 0.0, 'premium'),
    ('kling-v2',                'kling-v2',              'kling',     'video',       'video', 'A', 0.280, 0.0, 'premium'),

    -- Luma AI
    ('luma-ray-2',              'ray-2',                 'lumaai',    'video',       'video', 'A', 0.180, 0.0, 'premium'),
    ('luma-ray-flash-2',        'ray-flash-2',           'lumaai',    'video',       'video', 'A', 0.080, 0.0, 'standard'),

    -- Pika
    ('pika-2-2',                'pika-2.2',              'pika',      'video',       'video', 'A', 0.120, 0.0, 'standard'),

    -- MiniMax Hailuo
    ('minimax-hailuo-02',       'hailuo-02',             'minimax',   'video',       'video', 'A', 0.100, 0.0, 'standard'),
    ('minimax-abab-6-5-chat',   'abab-6.5-chat',         'minimax',   'text',        'text',  'A', 0.001, 0.005, 'standard'),

    -- ByteDance Seedance
    ('bytedance-seedance-1-0-pro',  'seedance-1.0-pro',  'bytedance', 'video',       'video', 'A', 0.180, 0.0, 'premium'),
    ('bytedance-seedance-1-0-lite', 'seedance-1.0-lite', 'bytedance', 'video',       'video', 'A', 0.060, 0.0, 'standard'),

    -- Hume AI (audio)
    ('hume-octave',           'octave',                'hume',      'audio',       'audio', 'A', 0.020, 0.0, 'standard'),
    ('hume-evi-3',            'evi-3',                 'hume',      'audio',       'audio', 'A', 0.060, 0.0, 'premium')
ON CONFLICT (id) DO NOTHING;
