// Main app layout - mobile-first, single-column with responsive sidebar
import { useAppStore, useActiveList, useLists } from "./store";
import { ListSelector } from "./components/ListSelector";
import { GroceryList } from "./components/GroceryList";
import { Header } from "./components/Header";
import "./styles/layout.css";

function App() {
  const activeListId = useAppStore((state) => state.activeListId);
  const activeList = useActiveList();
  const lists = useLists();

  return (
    <div className="app-container">
      <Header />

      <main className="app-main">
        {/* Mobile: show list selector or active list */}
        {/* Desktop: show both side-by-side */}
        <aside className="sidebar" aria-label="Your lists">
          <ListSelector lists={lists} activeListId={activeListId} />
        </aside>

        <section className="content" aria-label="Grocery list">
          {activeList ? (
            <GroceryList list={activeList} />
          ) : (
            <div className="empty-state">
              <p>Select a list or create a new one to get started.</p>
            </div>
          )}
        </section>
      </main>
    </div>
  );
}

export default App;
