import { FilterBar } from "../FilterBar";
import { HistoryList } from "../HistoryList";
import { HistoryItem } from "../../types";

interface HomeViewProps {
  // FilterBar props
  activeTab: string;
  onTabChange: (id: string) => void;
  onScrollToTop: () => void;
  // HistoryList props (pass-through)
  filteredHistory: HistoryItem[];
  renderedHistory: HistoryItem[];
  renderTab: string;
  listStyle: { opacity: number; transform: string; transition: string };
  searchText: string;
  currentView: string;
  isQueueMode: boolean;
  queueIds: number[];
  editingId: number | null;
  editValue: string;
  setEditValue: React.Dispatch<React.SetStateAction<string>>;
  inputRef: React.RefObject<HTMLInputElement | null>;
  translations: Record<number, string>;
  isRefreshing: boolean;
  pullIndicatorRef: React.RefObject<HTMLDivElement | null>;
  pullIconRef: React.RefObject<HTMLDivElement | null>;
  pullTextRef: React.RefObject<HTMLSpanElement | null>;
  handlePullMouseDown: (e: React.MouseEvent, listRef: React.RefObject<HTMLDivElement | null>) => void;
  handlePullMouseMove: (e: React.MouseEvent, listRef: React.RefObject<HTMLDivElement | null>) => void;
  handlePullMouseUp: () => void;
  handlePullMouseLeave: () => void;
  handleCopy: (item: HistoryItem) => void;
  handleDelete: (e: React.MouseEvent, id: number) => void;
  handleOpenViewer: (e: React.MouseEvent, item: HistoryItem) => void;
  handleToggleTranslate: (e: React.MouseEvent, item: HistoryItem) => void;
  handleStartEdit: (e: React.MouseEvent, item: HistoryItem) => void;
  handleSaveEdit: (id: number) => void;
  handleKeyDown: (e: React.KeyboardEvent, id: number) => void;
  toggleCategory: (e: React.MouseEvent, item: HistoryItem) => void;
  // Scroll registration
  onRegisterScrollToTop?: (fn: () => void) => void;
}

export function HomeView(props: HomeViewProps) {
  return (
    <>
      <FilterBar
        activeTab={props.activeTab}
        onTabChange={props.onTabChange}
        onScrollToTop={props.onScrollToTop}
      />
      <HistoryList
        filteredHistory={props.filteredHistory}
        renderedHistory={props.renderedHistory}
        renderTab={props.renderTab}
        listStyle={props.listStyle}
        searchText={props.searchText}
        currentView={props.currentView}
        isQueueMode={props.isQueueMode}
        queueIds={props.queueIds}
        editingId={props.editingId}
        editValue={props.editValue}
        setEditValue={props.setEditValue}
        inputRef={props.inputRef}
        translations={props.translations}
        isRefreshing={props.isRefreshing}
        pullIndicatorRef={props.pullIndicatorRef}
        pullIconRef={props.pullIconRef}
        pullTextRef={props.pullTextRef}
        handlePullMouseDown={props.handlePullMouseDown}
        handlePullMouseMove={props.handlePullMouseMove}
        handlePullMouseUp={props.handlePullMouseUp}
        handlePullMouseLeave={props.handlePullMouseLeave}
        handleCopy={props.handleCopy}
        handleDelete={props.handleDelete}
        handleOpenViewer={props.handleOpenViewer}
        handleToggleTranslate={props.handleToggleTranslate}
        handleStartEdit={props.handleStartEdit}
        handleSaveEdit={props.handleSaveEdit}
        handleKeyDown={props.handleKeyDown}
        toggleCategory={props.toggleCategory}
        onRegisterScrollToTop={props.onRegisterScrollToTop}
      />
    </>
  );
}
