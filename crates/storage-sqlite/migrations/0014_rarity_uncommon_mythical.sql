UPDATE rarities SET name = 'mythical', display_name = 'Mythical', image = 'mythical.png' WHERE name = 'epic';

INSERT INTO rarities (id, name, display_name, image, color)
SELECT 5, 'uncommon', 'Uncommon', 'uncommon.png', '#64B5F6'
WHERE NOT EXISTS (SELECT 1 FROM rarities WHERE name = 'uncommon');
