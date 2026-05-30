export interface Cap {
  id: string;
  name: string;
  description: string;
  password?: string;
  boardIds: string[];
}

export interface NgWord {
  id: string;
  name: string;
  word: string;
  boardIds: string[];
}
