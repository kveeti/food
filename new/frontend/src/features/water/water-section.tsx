import { WaterEntryList } from "./water-entry-list.tsx";
import { WaterForm } from "./water-form.tsx";
import { WaterTotal } from "./water-total.tsx";

export function WaterSection(props: { date: string }) {
  return (
    <section
      aria-labelledby="water-heading"
      className="mt-10 border-t border-gray-200 pt-6"
    >
      <header className="flex items-baseline justify-between gap-4">
        <h2 id="water-heading" className="text-lg font-medium text-gray-950">
          Water
        </h2>
        <WaterTotal date={props.date} />
      </header>
      <WaterForm date={props.date} />
      <WaterEntryList date={props.date} />
    </section>
  );
}
