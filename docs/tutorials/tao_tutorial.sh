#!/bin/bash
# Karduun CLI Tutorial: Tao
#
# This script demonstrates how to use the Karduun CLI suite to create cards
# and a dynamic deck for "Taoism" using a REPL-friendly, Bash-first workflow.
#
# Usage:
#   ./tao_tutorial.sh
#

set -euo pipefail

# Source shared helper functions
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/tutorial_helpers.sh"

# ============================================================================
# STEP 1: Initialize Workspace (we assume tools are installed)
# ============================================================================
echo_section "Step 1: Initializing Workspace"

WORKDIR=${WORKDIR:-tao}

if [ -d "$WORKDIR" ]; then
    echo_warn "Directory '$WORKDIR' already exists."
    rm -rf "$WORKDIR"
fi

mkdir -p "$WORKDIR"
echo_success "Created workspace directory: $WORKDIR"

cd "$WORKDIR"

echo_info "Initializing cardstack repository..."
scribe init
echo_success "Repository initialized"

echo ""

# ============================================================================
# STEP 2: Create Templates for Tao Concepts and People
# ============================================================================
echo_section "Step 2: Creating Templates"

echo_info "Creating concept template..."
stencil new "Tao Concept" \
  --required-field "fields.definition" \
  --required-field "fields.significance" \
  --required-field "fields.lesson" \
  --enum-field "fields.category=philosophy,cosmology,ethics,practice"

echo_success "Created Tao Concept template"

echo_info "Creating person template..."
stencil new "Tao Person" \
  --required-field "fields.birth_year" \
  --required-field "fields.contributions" \
  --required-field "fields.bio" \
  --enum-field "fields.role=philosopher,teacher,writer,leader"

echo_success "Created Tao Person template"

echo ""

# ============================================================================
# STEP 3: Create Example Cards from Templates
# ============================================================================
echo_section "Step 3: Creating Example Cards"

echo_info "Creating Tao cards..."

mkdir import
# Create JSONL files for each card
cat > import/cards.jsonl << 'EOF'
{"type": "card", "kind": "card", "schema": 1, "title": "Tao", "template": "template-tao-concept", "fields": {"definition": "The fundamental principle and source of all existence", "significance": "Central concept representing the natural order of the universe", "lesson": "The Tao teaches us to embrace simplicity and spontaneity. By aligning with the natural flow of life, we can achieve harmony and balance. It encourages us to let go of rigid expectations and instead follow the path of least resistance, allowing life to unfold naturally.", "category": "philosophy"}, "tags": ["taoism", "concept"], "facets": {"content": {"body": "The Tao, often translated as 'The Way,' is the central concept in Taoism, a philosophical and religious tradition originating in ancient China. It represents the natural order of the universe and the source of all existence. The Tao is described as an ineffable and transcendent force that flows through all things, guiding the processes of nature and the cosmos. It emphasizes the importance of living in harmony with this natural flow, embracing simplicity, spontaneity, and humility. The Tao is not a static entity but a dynamic process, constantly changing and adapting. It is often associated with water, which is soft and yielding yet powerful and persistent. The Tao Te Ching, attributed to Laozi, is the foundational text that explores the nature of the Tao and its application to human life. The Tao is often described as the underlying principle that governs the universe, encompassing both the physical and metaphysical realms. It is the source of all creation and the ultimate reality that underlies all phenomena. The Tao is not something that can be fully understood or defined through language or logic, as it transcends human comprehension. Instead, it is experienced through intuition, meditation, and a deep connection with nature. The Tao Te Ching teaches that the Tao is the mother of all things, nurturing and sustaining the universe in a state of perpetual balance and harmony. By aligning with the Tao, individuals can achieve a state of inner peace, wisdom, and spiritual enlightenment."}}}
{"type": "card", "kind": "card", "schema": 1, "title": "Wuwei", "template": "template-tao-concept", "fields": {"definition": "Action through non-action, effortless doing", "significance": "Key principle of aligning with the natural flow of life", "lesson": "Wu Wei teaches us the art of effortless action. It is not about inaction but about acting in harmony with the natural order. By letting go of unnecessary struggle and forcing outcomes, we can achieve more with less effort, leading to a life of ease and fulfillment.", "category": "practice"}, "tags": ["taoism", "concept"], "facets": {"content": {"body": "Wu Wei is a fundamental principle in Taoism that translates to 'non-action' or 'effortless action.' It does not imply inaction or laziness but rather emphasizes the importance of acting in harmony with the natural flow of life. Wu Wei encourages individuals to let go of excessive control, forcing, and struggle, and instead align their actions with the natural order of the universe. This principle is often illustrated through the metaphor of water, which effortlessly flows around obstacles and adapts to its surroundings. By practicing Wu Wei, one can achieve greater efficiency, reduce stress, and cultivate a sense of ease and fulfillment. It is about knowing when to act and when to refrain, allowing life to unfold naturally without unnecessary interference. The concept of Wu Wei is deeply rooted in the idea of spontaneity and naturalness. It suggests that the best way to achieve one's goals is not through force or manipulation but by working with the natural rhythms of life. This approach is often contrasted with the Western emphasis on control and domination, highlighting the Taoist belief in the power of yielding and adaptability. Wu Wei is not about passivity but about acting in a way that is effortless and in tune with the Tao. It is a state of being where actions are performed without attachment to outcomes, allowing individuals to flow with life's changes and challenges. This principle is applied in various aspects of life, from personal relationships to governance, emphasizing the importance of flexibility, patience, and trust in the natural order."}}}
{"type": "card", "kind": "card", "schema": 1, "title": "Yinyang", "template": "template-tao-concept", "fields": {"definition": "Complementary forces that interact to form a dynamic system", "significance": "Represents balance and interdependence in all things", "lesson": "Yin Yang reminds us that life is a dynamic interplay of opposing forces. By understanding and embracing these dualities, we can achieve balance and harmony. It teaches us to appreciate the interconnectedness of all things and to seek equilibrium in our actions and thoughts.", "category": "cosmology"}, "tags": ["taoism", "concept"], "facets": {"content": {"body": "Yin Yang is a fundamental concept in Chinese philosophy and cosmology, representing the dualistic nature of the universe. Yin and Yang are complementary forces that interact to form a dynamic system, where each force contains the seed of its opposite. Yin is associated with qualities such as darkness, passivity, femininity, and cold, while Yang is associated with light, activity, masculinity, and warmth. Together, they symbolize the interconnectedness and interdependence of all things. The Yin Yang symbol, known as the Taijitu, illustrates this balance, with each side containing a small circle of the opposite color, signifying that one cannot exist without the other. This concept is applied to various aspects of life, including health, relationships, and the natural world, emphasizing the importance of balance and harmony. The philosophy of Yin Yang is deeply embedded in traditional Chinese medicine, martial arts, and various cultural practices. It teaches that health and well-being are achieved through the balance of these opposing forces within the body and mind. In martial arts, the principle of Yin Yang is used to understand the dynamics of movement and energy, emphasizing the interplay between softness and hardness, and between yielding and asserting. In daily life, the concept encourages individuals to recognize and embrace the dualities within themselves and their surroundings, fostering a holistic and integrated approach to living. The dynamic interplay of Yin and Yang is seen as the driving force behind all change and transformation in the universe."}}}
EOF

# Append philosopher cards to the JSONL file
cat >> import/cards.jsonl << 'EOF'
{"type": "card", "kind": "card", "schema": 1, "title": "Laozi", "template": "template-tao-person", "fields": {"birth_year": -600, "contributions": "Author of Tao Te Ching, founder of Taoism", "bio": "Laozi, also known as Lao Tzu, is a semi-legendary figure in Chinese history. Traditionally regarded as the author of the Tao Te Ching, he is the founder of Taoism. His teachings emphasize living in harmony with the Tao, the natural order of the universe. Little is known about his life, but his philosophical contributions have had a profound and lasting impact on Chinese thought and culture.", "role": "philosopher"}, "tags": ["taoism", "philosopher"], "facets": {"content": {"body": "Laozi, also known as Lao Tzu, is a central figure in Chinese philosophy and the traditional founder of Taoism. He is best known as the author of the Tao Te Ching, a foundational text that explores the nature of the Tao and its application to human life. According to tradition, Laozi lived in the 6th century BCE and served as a record-keeper in the court of the Zhou dynasty. Disillusioned with the corruption and decline of the dynasty, he reportedly left civilization and rode westward on a water buffalo. At the request of a border guard, he wrote down his teachings, which became the Tao Te Ching. The text emphasizes the importance of living in harmony with the Tao, or the natural order of the universe, and advocates for simplicity, humility, and non-action. Laozi's teachings have had a profound influence on Chinese thought, culture, and spirituality, and continue to inspire people around the world. Laozi's philosophy is characterized by its emphasis on the natural way of life and the rejection of artificial constructs and conventions. The Tao Te Ching is a collection of poetic and aphoristic verses that offer guidance on how to live in accordance with the Tao. It teaches that true wisdom comes from understanding the interconnectedness of all things and aligning one's actions with the natural flow of the universe. Laozi's ideas have influenced not only Taoism but also other schools of Chinese thought, including Confucianism and Buddhism. His teachings on governance, ethics, and personal conduct have been studied and revered for centuries, making him one of the most significant figures in the history of Chinese philosophy."}}}
{"type": "card", "kind": "card", "schema": 1, "title": "Zhuangzi", "template": "template-tao-person", "fields": {"birth_year": -369, "contributions": "Major Taoist text Zhuangzi, developed Taoist philosophy", "bio": "Zhuangzi, also known as Chuang Tzu, was an influential Chinese philosopher who lived around the 4th century BCE. He is best known for the book that bears his name, the Zhuangzi, which is one of the foundational texts of Taoism. His writings are filled with parables, anecdotes, and humor, emphasizing the importance of living spontaneously and in accordance with the natural world.", "role": "philosopher"}, "tags": ["taoism", "philosopher"], "facets": {"content": {"body": "Zhuangzi, also known as Chuang Tzu, was a prominent Chinese philosopher who lived during the 4th century BCE. He is best known for his contributions to Taoist philosophy, particularly through the text that bears his name, the Zhuangzi. This work is a collection of essays, parables, and anecdotes that explore themes such as the relativity of knowledge, the importance of living in harmony with nature, and the pursuit of spiritual freedom. Zhuangzi's writings are characterized by their humor, wit, and imaginative storytelling, often using allegories and metaphors to convey complex philosophical ideas. One of his most famous parables is the 'Butterfly Dream,' in which he questions the nature of reality and the distinction between the self and the external world. Zhuangzi's philosophy emphasizes the importance of spontaneity, flexibility, and the rejection of rigid conventions, advocating for a life that is in tune with the natural order of the universe. Zhuangzi's philosophy is deeply rooted in the idea of naturalness and the rejection of artificial distinctions and societal norms. He believed that true happiness and freedom come from living in accordance with the Tao, the natural way of the universe. His writings often challenge conventional wisdom and encourage individuals to see beyond the limitations of human perception and language. Zhuangzi's ideas have had a lasting impact on Chinese thought and culture, influencing not only Taoism but also other philosophical and spiritual traditions. His emphasis on the interconnectedness of all things and the relativity of human knowledge continues to resonate with readers and thinkers around the world."}}}
{"type": "card", "kind": "card", "schema": 1, "title": "Liezi", "template": "template-tao-person", "fields": {"birth_year": -400, "contributions": "Daoist text Liezi, emphasized harmony with nature", "bio": "Liezi, also known as Lieh Tzu, was a Taoist philosopher believed to have lived around the 4th century BCE. He is traditionally credited with authoring the Daoist text Liezi, which explores themes of harmony with nature, detachment from worldly concerns, and the pursuit of spiritual freedom. His teachings emphasize the importance of living a simple and natural life.", "role": "philosopher"}, "tags": ["taoism", "philosopher"], "facets": {"content": {"body": "Liezi, also known as Lieh Tzu, was a Taoist philosopher who is traditionally believed to have lived around the 4th century BCE. He is best known for the text that bears his name, the Liezi, which is one of the foundational works of Taoism. The Liezi explores themes such as the nature of reality, the importance of living in harmony with nature, and the pursuit of spiritual freedom. The text is a collection of stories, dialogues, and philosophical reflections that emphasize the virtues of simplicity, detachment, and non-action. One of the most famous stories in the Liezi is the tale of the 'Man Who Could Not Be Harmed,' which illustrates the power of inner peace and resilience. Liezi's teachings encourage individuals to let go of worldly concerns and align themselves with the natural order of the universe, cultivating a life of balance, harmony, and spiritual fulfillment. Liezi's philosophy is deeply influenced by the ideas of naturalness and the rejection of artificial constructs. He believed that true wisdom and happiness come from living in accordance with the Tao, the natural way of the universe. His teachings emphasize the importance of detachment from worldly desires and the cultivation of inner peace and spiritual resilience. The Liezi is a rich and diverse text that includes stories, fables, and philosophical discussions, offering insights into the nature of reality and the human condition. Liezi's ideas have had a significant impact on the development of Taoist thought and continue to inspire readers with their emphasis on simplicity, harmony, and the pursuit of spiritual enlightenment."}}}
EOF

# Import all cards using porter
porter import --from jsonl --in import

echo_success "Created Tao cards"

echo ""

# ============================================================================
# STEP 4: Create and Populate Deck
# ============================================================================
echo_section "Step 4: Creating Taoism Deck"

echo_info "Creating Taoism deck..."
scribe deck new "Taoism Fundamentals" --mode static

echo_info "Adding concept cards to deck..."
scribe deck add "taoism-fundamentals" "tao" "wuwei" "yinyang"

echo_info "Adding philosopher cards to deck..."
scribe deck add "taoism-fundamentals" "laozi" "zhuangzi" "liezi"

echo_success "Created and populated Taoism deck"

echo ""

# ============================================================================
# STEP 5: Publish Cards and Manage Albums
# ============================================================================
echo_section "Step 5: Publishing Cards and Managing Albums"

echo_info "Creating a new album for published cards..."
album create "Published Taoism"

echo_info "Publishing cards to the album..."
publisher publish "Published Taoism" "tao"
publisher publish "Published Taoism" "wuwei"
publisher publish "Published Taoism" "yinyang"
publisher publish "Published Taoism" "laozi"
publisher publish "Published Taoism" "zhuangzi"
publisher publish "Published Taoism" "liezi"

echo_info "Listing publications for a card..."
publisher list "tao"

echo_info "Showing cards in the published album..."
album show "Published Taoism"

echo_success "Published cards and managed albums successfully!"

echo ""

# ============================================================================
# STEP 6: Show Results
# ============================================================================
echo_section "Step 6: Viewing Results"

echo_info "Displaying deck contents..."
scribe deck show "taoism-fundamentals"

echo ""
echo_success "Taoism tutorial completed successfully!"
echo_info "You can explore the created cards, deck, and albums in the $WORKDIR directory."
