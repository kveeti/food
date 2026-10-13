import { Temporal } from "@js-temporal/polyfill";
import { AxisBottom, AxisLeft } from "@visx/axis";
import { curveMonotoneX } from "@visx/curve";
import { GridRows } from "@visx/grid";
import { Group } from "@visx/group";
import { ParentSize } from "@visx/responsive";
import { scaleLinear, scaleTime } from "@visx/scale";
import { AreaClosed, LinePath } from "@visx/shape";
import {
  memo,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  type KeyboardEvent,
  type PointerEvent,
} from "react";
import { createPortal } from "react-dom";

import { type WeightPoint, useWeightChartQuery } from "../../api/weight.ts";
import { Button } from "../../ui/button/button.tsx";
import { useI18n } from "../i18n/use-i18n.tsx";

type Range = "1M" | "3M" | "6M" | "1Y" | "All";

type WeightDatum = {
  date: Date;
  weight: number;
};

const ranges: Range[] = ["1M", "3M", "6M", "1Y", "All"];
const rangeLabels: Record<Range, string> = {
  "1M": "1-month",
  "3M": "3-month",
  "6M": "6-month",
  "1Y": "1-year",
  All: "All-time",
};
const dayMilliseconds = 24 * 60 * 60 * 1_000;
const chartLayout = {
  topPadding: 12,
  bottomPadding: 32,
  plotLeftInset: 32,
  svgRightPadding: 8,
  yAxisLabelOffset: 16,
  maxDateTicks: 5,
  minDateTickSpacing: 64,
  tooltipHalfWidth: 64,
  maxDataPoints: 200,
} as const;

export function WeightChart() {
  const { f, locale, timeZone } = useI18n();
  const [range, setRange] = useState<Range>("3M");
  const query = useWeightChartQuery(range, timeZone, chartLayout.maxDataPoints);
  const entries = query.data?.points ?? [];
  const isTransitioning = query.isPlaceholderData;
  const first = query.data?.first;
  const last = query.data?.last;
  const latest = query.data?.latest;
  const change = first && last ? last.weight_kg - first.weight_kg : null;

  return (
    <section aria-label="Weight">
      <div
        aria-busy={query.isPending || isTransitioning}
        className={`transition-[filter,opacity] duration-200 ${isTransitioning ? "opacity-70 blur-[1px]" : ""}`}
      >
        {query.isPending ? (
          <div aria-hidden="true">
            <div className="mt-0.5 h-[1.8rem] w-28 animate-pulse rounded bg-gray-200" />
            <div className="mt-1 h-5 w-44 animate-pulse rounded bg-gray-200" />
          </div>
        ) : (
          <>
            <div className="mt-0.5 text-title font-semibold tabular-nums text-gray-950">
              {latest
                ? `${f.amount(latest.weight_kg)} kg`
                : query.isError
                  ? "Could not load weight"
                  : "No weight yet"}
            </div>
            <div className="mt-1 h-5 text-sm text-gray-600">
              {latest && (
                <time dateTime={latest.measured_at}>
                  {measurementTimeLabel(latest.measured_at, timeZone, f)}
                </time>
              )}
            </div>
          </>
        )}
      </div>

      <div
        role="group"
        aria-label="Weight chart range"
        className="mt-5 grid grid-cols-5 rounded-xl border border-gray-200 bg-gray-100/85 p-1 text-gray-800 shadow-float backdrop-blur-xl"
      >
        {ranges.map((value) => (
          <button
            key={value}
            type="button"
            aria-pressed={range === value}
            onClick={() => setRange(value)}
            className="cursor-pointer rounded-lg px-2 py-1.5 text-sm font-medium outline-2 outline-transparent outline-offset-[-2px] hover:text-gray-950 focus-visible:outline-gray-500 aria-pressed:bg-gray-250"
          >
            {value}
          </button>
        ))}
      </div>

      <div
        aria-busy={query.isPending || isTransitioning}
        className={`mt-4 transition-[filter,opacity] duration-200 ${isTransitioning ? "opacity-70 blur-[1px]" : ""}`}
      >
        {query.isError && query.data && (
          <div className="mb-3 flex items-center justify-between gap-3 text-sm text-danger-fg">
            <p role="alert">Could not refresh weight.</p>
            <Button
              type="button"
              variant="ghost"
              onClick={() => void query.refetch()}
            >
              Try again
            </Button>
          </div>
        )}
        {query.isError && !query.data ? (
          <div className="flex min-h-72 items-center justify-center gap-3 px-4">
            <p role="alert" className="text-sm text-danger-fg">
              Could not load weight.
            </p>
            <Button
              type="button"
              variant="ghost"
              onClick={() => void query.refetch()}
            >
              Try again
            </Button>
          </div>
        ) : query.isPending ? (
          <div aria-hidden="true" className="min-h-[19.25rem]">
            <div className="flex h-4 items-center justify-between">
              <div className="h-3 w-28 animate-pulse rounded bg-gray-200" />
              <div className="h-3 w-32 animate-pulse rounded bg-gray-200" />
            </div>
            <div className="mt-1 h-72 animate-pulse rounded-2xl bg-gray-100" />
          </div>
        ) : entries.length === 0 ? (
          <div className="grid min-h-[19.25rem] place-items-center px-4 text-center">
            <p className="text-sm text-gray-600">
              {range === "All"
                ? "Add a weight to start your chart."
                : `No weights in this ${range.toLowerCase()} range.`}
            </p>
          </div>
        ) : (
          <>
            <div className="flex flex-wrap items-center justify-between gap-3 text-xs">
              <p>
                <span className="text-gray-600">
                  {rangeLabels[range]} change
                </span>{" "}
                <span className="font-medium tabular-nums text-gray-950">
                  {change === null
                    ? "—"
                    : `${change > 0 ? "+" : change < 0 ? "−" : ""}${f.amount(Math.abs(change))} kg`}
                </span>
              </p>
              <div className="flex items-center gap-4 text-gray-600">
                <ChartKey color="var(--color-gray-500)" label="Weight" />
                <ChartKey color="var(--color-green-650)" label="Trend" />
              </div>
            </div>
            <div className="mt-1 h-72 min-w-0 overflow-visible">
              <ParentSize
                debounceTime={80}
                style={{
                  width: `calc(100% + ${chartLayout.svgRightPadding}px)`,
                }}
              >
                {({ width, height }) =>
                  width > 0 && height > 0 ? (
                    <WeightPlot
                      key={`${range}:${entries.at(-1)?.measured_at}`}
                      width={width}
                      height={height}
                      entries={entries}
                      locale={locale}
                      timeZone={timeZone}
                    />
                  ) : null
                }
              </ParentSize>
            </div>
          </>
        )}
      </div>
    </section>
  );
}

function ChartKey(props: { color: string; label: string }) {
  return (
    <span className="flex items-center gap-1.5">
      <span
        aria-hidden="true"
        className="h-0.5 w-4 rounded-full"
        style={{ background: props.color }}
      />
      {props.label}
    </span>
  );
}

function WeightPlot(props: {
  width: number;
  height: number;
  entries: WeightPoint[];
  locale: string;
  timeZone: string;
}) {
  const [selectedIndex, setSelectedIndex] = useState<number | null>(null);
  const [tooltipPosition, setTooltipPosition] = useState<{
    top: number;
    left: number;
  } | null>(null);
  const data = useMemo(
    () =>
      props.entries.map((entry) => ({
        date: new Date(entry.measured_at),
        weight: entry.weight_kg,
      })),
    [props.entries],
  );
  const trend = useMemo(
    () =>
      props.entries.map((entry) => ({
        date: new Date(entry.measured_at),
        weight: entry.trend_weight_kg,
      })),
    [props.entries],
  );
  const innerWidth = Math.max(0, props.width - chartLayout.svgRightPadding);
  const innerHeight = Math.max(
    0,
    props.height - chartLayout.topPadding - chartLayout.bottomPadding,
  );
  const dateTicks = useMemo(
    () =>
      spacedDateTicks(
        trend,
        Math.max(
          2,
          Math.min(
            chartLayout.maxDateTicks,
            Math.floor(innerWidth / chartLayout.minDateTickSpacing),
          ),
        ),
        props.timeZone,
      ),
    [innerWidth, props.timeZone, trend],
  );
  const [dataXScale, yScale] = useMemo(() => {
    const dates = data.map((datum) => datum.date.valueOf());
    const weights = [...data, ...trend].map((datum) => datum.weight);
    const [dateMin, dateMax] = dateDomain(
      Math.min(...dates),
      Math.max(...dates),
    );
    const [weightMin, weightMax] = paddedWeightDomain(
      Math.min(...weights),
      Math.max(...weights),
    );
    const leftInset = Math.min(chartLayout.plotLeftInset, innerWidth / 2);
    return [
      createDateScale(dateMin, dateMax, [leftInset, innerWidth]),
      createWeightScale(weightMin, weightMax, innerHeight),
    ] as const;
  }, [data, innerHeight, innerWidth, trend]);
  const selected = selectedIndex === null ? null : data[selectedIndex];
  const svgRef = useRef<SVGSVGElement>(null);
  const dateFormat = useMemo(
    () =>
      new Intl.DateTimeFormat(props.locale, {
        month: "short",
        day: "numeric",
        timeZone: props.timeZone,
      }),
    [props.locale, props.timeZone],
  );
  const tooltipDateFormat = useMemo(
    () =>
      new Intl.DateTimeFormat(props.locale, {
        dateStyle: "medium",
        timeStyle: "short",
        timeZone: props.timeZone,
      }),
    [props.locale, props.timeZone],
  );
  const numberFormat = useMemo(
    () =>
      new Intl.NumberFormat(props.locale, {
        maximumFractionDigits: 2,
      }),
    [props.locale],
  );
  function selectFromPointer(event: PointerEvent<SVGRectElement>) {
    const bounds = event.currentTarget.getBoundingClientRect();
    const x = Math.max(0, Math.min(innerWidth, event.clientX - bounds.left));
    const time = dataXScale.invert(x).valueOf();
    setSelectedIndex(nearestIndex(data, time));
  }

  function moveSelection(event: KeyboardEvent<SVGSVGElement>) {
    if (event.key !== "ArrowLeft" && event.key !== "ArrowRight") return;
    event.preventDefault();
    const direction = event.key === "ArrowLeft" ? -1 : 1;
    setSelectedIndex((index) =>
      Math.max(
        0,
        Math.min(data.length - 1, (index ?? data.length - 1) + direction),
      ),
    );
  }

  useLayoutEffect(() => {
    if (!selected || !svgRef.current) return;
    setTooltipPosition(
      getTooltipPosition(svgRef.current, dataXScale(selected.date)),
    );
  }, [dataXScale, selected]);

  useEffect(() => {
    if (!selected) return;
    const closeTooltip = () => setSelectedIndex(null);
    window.addEventListener("scroll", closeTooltip, true);
    return () => window.removeEventListener("scroll", closeTooltip, true);
  }, [selected]);

  return (
    <div className="relative h-full w-full">
      <svg
        ref={svgRef}
        width={props.width}
        height={props.height}
        role="img"
        aria-label="Weight chart. Use the left and right arrow keys to inspect measurements."
        tabIndex={0}
        onFocus={() => setSelectedIndex((index) => index ?? data.length - 1)}
        onBlur={() => setSelectedIndex(null)}
        onKeyDown={moveSelection}
        className="block touch-pan-y overflow-visible rounded-2xl outline-2 outline-transparent outline-offset-[-2px] focus-visible:outline-gray-500"
      >
        <desc>Measurements and their seven-day moving average.</desc>
        <defs>
          <linearGradient id="weight-trend-fill" x1="0" y1="0" x2="0" y2="1">
            <stop
              offset="0%"
              stopColor="var(--color-green-550)"
              stopOpacity="0.2"
            />
            <stop
              offset="100%"
              stopColor="var(--color-green-550)"
              stopOpacity="0"
            />
          </linearGradient>
        </defs>
        <Group top={chartLayout.topPadding}>
          <StaticWeightPlot
            data={data}
            trend={trend}
            width={innerWidth}
            height={innerHeight}
            dateTicks={dateTicks}
            dataXScale={dataXScale}
            yScale={yScale}
            dateFormat={dateFormat}
            numberFormat={numberFormat}
          />
          {selected && (
            <>
              <line
                x1={dataXScale(selected.date)}
                x2={dataXScale(selected.date)}
                y1={0}
                y2={innerHeight}
                stroke="var(--color-gray-400)"
                strokeDasharray="3 4"
              />
              <circle
                cx={dataXScale(selected.date)}
                cy={yScale(selected.weight)}
                r={5}
                fill="var(--color-gray-50)"
                stroke="var(--color-green-650)"
                strokeWidth={3}
              />
            </>
          )}
          <rect
            width={innerWidth}
            height={innerHeight}
            fill="transparent"
            onPointerDown={selectFromPointer}
            onPointerMove={(event) => {
              if (event.pointerType === "mouse" || event.buttons === 1) {
                selectFromPointer(event);
              }
            }}
            onPointerLeave={(event) => {
              if (event.pointerType === "mouse") setSelectedIndex(null);
            }}
          />
        </Group>
      </svg>

      {selected && tooltipPosition
        ? createPortal(
            <div
              role="status"
              aria-label="Selected weight"
              className="pointer-events-none absolute z-10 min-w-28 -translate-x-1/2 rounded-xl border border-popover-border bg-popover/95 px-3 py-2 text-center shadow-float backdrop-blur-md"
              style={tooltipPosition}
            >
              <div className="font-semibold tabular-nums text-gray-950">
                {numberFormat.format(selected.weight)} kg
              </div>
              <div className="mt-0.5 whitespace-nowrap text-xs text-gray-600">
                {tooltipDateFormat.format(selected.date)}
              </div>
            </div>,
            document.body,
          )
        : null}
    </div>
  );
}

const StaticWeightPlot = memo(function StaticWeightPlot(props: {
  data: WeightDatum[];
  trend: WeightDatum[];
  width: number;
  height: number;
  dateTicks: Date[];
  dataXScale: ReturnType<typeof createDateScale>;
  yScale: ReturnType<typeof createWeightScale>;
  dateFormat: Intl.DateTimeFormat;
  numberFormat: Intl.NumberFormat;
}) {
  return (
    <>
      <GridRows
        scale={props.yScale}
        left={0}
        width={props.width}
        numTicks={4}
        stroke="var(--color-gray-250)"
        strokeDasharray="2 4"
      />
      <AreaClosed
        data={props.trend}
        x={(datum) => props.dataXScale(datum.date) ?? 0}
        y={(datum) => props.yScale(datum.weight) ?? 0}
        yScale={props.yScale}
        curve={curveMonotoneX}
        fill="url(#weight-trend-fill)"
      />
      <LinePath
        data={props.data}
        x={(datum) => props.dataXScale(datum.date) ?? 0}
        y={(datum) => props.yScale(datum.weight) ?? 0}
        stroke="var(--color-gray-500)"
        strokeWidth={1.25}
        strokeLinecap="round"
        strokeLinejoin="round"
      />
      <LinePath
        data={props.trend}
        x={(datum) => props.dataXScale(datum.date) ?? 0}
        y={(datum) => props.yScale(datum.weight) ?? 0}
        curve={curveMonotoneX}
        stroke="var(--color-green-650)"
        strokeWidth={3}
        strokeLinecap="round"
        strokeLinejoin="round"
      />
      {props.data.length === 1 && (
        <circle
          cx={props.dataXScale(props.data[0].date)}
          cy={props.yScale(props.data[0].weight)}
          r={4}
          fill="var(--color-green-650)"
        />
      )}
      <AxisLeft
        scale={props.yScale}
        numTicks={4}
        hideAxisLine
        hideTicks
        tickFormat={(value) => props.numberFormat.format(Number(value))}
        tickLabelProps={{
          fill: "var(--color-gray-600)",
          fontFamily: "inherit",
          fontSize: 11,
          textAnchor: "start",
          dx: chartLayout.yAxisLabelOffset,
          dy: 3,
        }}
      />
      <AxisBottom
        top={props.height}
        scale={props.dataXScale}
        tickValues={props.dateTicks}
        hideAxisLine
        hideTicks
        tickFormat={(value) =>
          props.dateFormat.format(new Date(value.valueOf()))
        }
        tickLabelProps={(value) => ({
          fill: "var(--color-gray-600)",
          fontFamily: "inherit",
          fontSize: 11,
          textAnchor:
            value.valueOf() === props.dateTicks.at(-1)?.valueOf()
              ? "end"
              : "middle",
          dy: 8,
        })}
      />
    </>
  );
});

function measurementTimeLabel(
  value: string,
  timeZone: string,
  format: {
    dateKey: (value: Date) => string;
    dateOnly: (value: string) => string;
    time: (value: Date) => string;
  },
) {
  const instant = Temporal.Instant.from(value);
  const measuredDate = instant.toZonedDateTimeISO(timeZone).toPlainDate();
  const today = Temporal.Now.instant()
    .toZonedDateTimeISO(timeZone)
    .toPlainDate();
  const day = measuredDate.equals(today)
    ? "Today"
    : measuredDate.equals(today.subtract({ days: 1 }))
      ? "Yesterday"
      : format.dateOnly(format.dateKey(new Date(value)));
  return `${day} · ${format.time(new Date(value))}`;
}

function createDateScale(min: number, max: number, range: [number, number]) {
  return scaleTime<number>({
    domain: [new Date(min), new Date(max)],
    range,
  });
}

function getTooltipPosition(svg: SVGSVGElement, x: number) {
  const bounds = svg.getBoundingClientRect();
  const pageX = bounds.left + window.scrollX + x;
  const minX = window.scrollX + chartLayout.tooltipHalfWidth;
  const maxX =
    window.scrollX + window.innerWidth - chartLayout.tooltipHalfWidth;
  return {
    top: bounds.top + window.scrollY,
    left: Math.min(Math.max(pageX, minX), maxX),
  };
}

function createWeightScale(min: number, max: number, height: number) {
  return scaleLinear<number>({
    domain: [min, max],
    range: [height, 0],
    nice: true,
  });
}

function dateDomain(min: number, max: number) {
  if (min === max)
    return [min - dayMilliseconds, max + dayMilliseconds] as const;
  return [min, max] as const;
}

function paddedWeightDomain(min: number, max: number) {
  const padding = Math.max((max - min) * 0.22, 0.5);
  return [min - padding, max + padding] as const;
}

function nearestIndex(data: WeightDatum[], time: number) {
  let nearest = 0;
  for (let index = 1; index < data.length; index += 1) {
    if (
      Math.abs(data[index].date.valueOf() - time) <
      Math.abs(data[nearest].date.valueOf() - time)
    ) {
      nearest = index;
    }
  }
  return nearest;
}

function spacedDateTicks(data: WeightDatum[], count: number, timeZone: string) {
  const first = data[0]?.date;
  const last = data.at(-1)?.date;
  if (!first || !last) return [];
  const firstTime = Temporal.Instant.fromEpochMilliseconds(
    first.getTime(),
  ).toZonedDateTimeISO(timeZone);
  const firstDay = firstTime.toPlainDate();
  const lastDay = Temporal.Instant.fromEpochMilliseconds(last.getTime())
    .toZonedDateTimeISO(timeZone)
    .toPlainDate();
  const dayCount = lastDay.since(firstDay).days;
  const tickCount = Math.min(count, dayCount + 1);
  if (tickCount === 1) return [first];
  return Array.from({ length: tickCount }, (_, index) => {
    if (index === 0) return first;
    if (index === tickCount - 1) return last;
    const day = firstDay.add({
      days: Math.floor((index * dayCount) / (tickCount - 1)),
    });
    return new Date(
      day.toZonedDateTime({
        timeZone,
        plainTime: firstTime.toPlainTime(),
      }).epochMilliseconds,
    );
  });
}
