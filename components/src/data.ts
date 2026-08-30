export type Fruit = {
  id: string;
  name: string;
  description: string;
  aliases: string[];
};

export const fruits: Fruit[] = [
  { id: "apple", name: "Apple", description: "Crisp, sweet, and good for everyday snacks.", aliases: ["pome"] },
  { id: "apricot", name: "Apricot", description: "Soft stone fruit with a light tart edge.", aliases: ["stone fruit"] },
  { id: "avocado", name: "Avocado", description: "Creamy fruit often used in savoury food.", aliases: ["alligator pear"] },
  { id: "banana", name: "Banana", description: "Soft, sweet, and easy to carry.", aliases: ["plantain"] },
  { id: "blackberry", name: "Blackberry", description: "Dark berry with a rich, tart taste.", aliases: ["bramble"] },
  { id: "blueberry", name: "Blueberry", description: "Small blue berry with mild sweetness.", aliases: ["bilberry"] },
  { id: "cherry", name: "Cherry", description: "Small stone fruit, from tart to very sweet.", aliases: ["stone fruit"] },
  { id: "clementine", name: "Clementine", description: "Small, sweet citrus with loose skin.", aliases: ["mandarin", "orange"] },
  { id: "grape", name: "Grape", description: "Juicy fruit that grows in small bunches.", aliases: ["vine fruit"] },
  { id: "grapefruit", name: "Grapefruit", description: "Large citrus with a bitter edge.", aliases: ["citrus"] },
  { id: "kiwi", name: "Kiwi", description: "Bright green fruit with tiny black seeds.", aliases: ["kiwifruit"] },
  { id: "lemon", name: "Lemon", description: "Sharp yellow citrus used for juice and zest.", aliases: ["citrus"] },
  { id: "mango", name: "Mango", description: "Rich tropical fruit with soft orange flesh.", aliases: ["tropical"] },
  { id: "nectarine", name: "Nectarine", description: "Smooth-skinned stone fruit like a peach.", aliases: ["stone fruit"] },
  { id: "orange", name: "Orange", description: "Juicy citrus with a sweet, bright taste.", aliases: ["citrus"] },
  { id: "peach", name: "Peach", description: "Soft, fragrant stone fruit with fuzzy skin.", aliases: ["stone fruit"] },
  { id: "pear", name: "Pear", description: "Sweet fruit with soft, lightly grainy flesh.", aliases: ["pome"] },
  { id: "pineapple", name: "Pineapple", description: "Large tropical fruit with sharp, sweet flesh.", aliases: ["tropical"] },
  { id: "plum", name: "Plum", description: "Smooth stone fruit with sweet-tart flesh.", aliases: ["stone fruit"] },
  { id: "raspberry", name: "Raspberry", description: "Delicate red berry with a tart finish.", aliases: ["berry"] },
  { id: "strawberry", name: "Strawberry", description: "Fragrant red berry with seeds on its skin.", aliases: ["berry"] },
];

export function findFruit(id: string | undefined): Fruit | undefined {
  return fruits.find((fruit) => fruit.id === id);
}

export function searchFruits(query: string): Fruit[] {
  const terms = query.toLocaleLowerCase().split(/\s+/).filter(Boolean);
  if (terms.length === 0) return [];

  return fruits
    .map((fruit) => {
      const name = fruit.name.toLocaleLowerCase();
      const text = [name, ...fruit.aliases].join(" ").toLocaleLowerCase();
      if (!terms.every((term) => text.includes(term))) return null;

      const score = name === terms.join(" ") ? 0 : name.startsWith(terms[0]) ? 1 : 2;
      return { fruit, score };
    })
    .filter((result): result is { fruit: Fruit; score: number } => result !== null)
    .sort((a, b) => a.score - b.score || a.fruit.name.localeCompare(b.fruit.name))
    .slice(0, 8)
    .map(({ fruit }) => fruit);
}
